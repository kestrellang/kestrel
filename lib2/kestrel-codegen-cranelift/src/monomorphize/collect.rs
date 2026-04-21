//! BFS instantiation collection.
//!
//! Discovers all function instantiations needed for compilation by walking
//! the call graph starting from non-generic entry points.
//!
//! Key differences from lib1:
//! - Types are by-value (`MirTy`), no interning needed
//! - Uses `substitute_type` from kestrel-codegen2
//! - Entity-based lookups via pre-built HashMap

use super::error::MonomorphizeError;
use super::instantiation::{FunctionInstantiation, MonomorphizationSet};
use super::witness;
use crate::common;
use kestrel_codegen2::{substitute_type, substitute_type_with_self};
use kestrel_debug::ktrace;
use kestrel_hecs::Entity;
use kestrel_mir::{Callee, FunctionId, FunctionKind, MirModule, MirTy, Rvalue, StatementKind};
use std::collections::{HashMap, VecDeque};

/// Collect all function instantiations needed for compilation.
pub fn collect_all(module: &MirModule) -> Result<MonomorphizationSet, Vec<MonomorphizeError>> {
    let mut ctx = CollectionContext::new(module);
    ctx.collect();

    if ctx.errors.is_empty() {
        Ok(ctx.result)
    } else {
        Err(ctx.errors)
    }
}

struct CollectionContext<'a> {
    module: &'a MirModule,
    /// O(1) entity → FunctionId lookup
    entity_to_func: HashMap<Entity, FunctionId>,
    /// BFS queue of instantiations to process
    queue: VecDeque<FunctionInstantiation>,
    /// Accumulated result
    result: MonomorphizationSet,
    /// Collected errors
    errors: Vec<MonomorphizeError>,
    /// Currently processing function (for debug)
    processing_func_id: Option<FunctionId>,
}

impl<'a> CollectionContext<'a> {
    fn new(module: &'a MirModule) -> Self {
        let entity_to_func: HashMap<Entity, FunctionId> = module
            .functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.entity, FunctionId::new(i)))
            .collect();

        Self {
            module,
            entity_to_func,
            queue: VecDeque::new(),
            result: MonomorphizationSet::new(),
            errors: Vec::new(),
            processing_func_id: None,
        }
    }

    fn collect(&mut self) {
        // Seed: all non-generic, non-closure entry points
        for (i, func) in self.module.functions.iter().enumerate() {
            if !func.type_params.is_empty() {
                continue;
            }

            // Check if function body references unresolved type params.
            // This happens for functions that should have inherited parent type
            // params but didn't (e.g., extension methods on generic types).
            // These are discovered through properly-resolved call sites.

            // Closures and thunks are always discovered through their parent
            // (ApplyPartial or FunctionRef), never seeded directly
            if matches!(
                func.kind,
                FunctionKind::ClosureCall { .. }
                    | FunctionKind::Closure
                    | FunctionKind::Thunk { .. }
            ) {
                continue;
            }

            if !self.func_uses_self_type(func) {
                // No SelfType — seed as concrete
                let inst = FunctionInstantiation::concrete(FunctionId::new(i));
                if self.result.functions.insert(inst.clone()) {
                    self.queue.push_back(inst);
                }
            } else {
                // Uses SelfType — for init/deinit/method of non-generic types,
                // resolve SelfType to the parent struct/enum type
                let parent = match &func.kind {
                    FunctionKind::Initializer { parent }
                    | FunctionKind::Deinit { parent }
                    | FunctionKind::Method { parent, .. }
                    | FunctionKind::StaticMethod { parent } => Some(*parent),
                    _ => None,
                };
                if let Some(parent_entity) = parent {
                    // Only seed if parent is a concrete type (struct/enum).
                    // Protocol extension methods have parent = Extension entity
                    // (not a type) — they're discovered through witness calls
                    // with concrete self_types at call sites.
                    let is_concrete_nongeneric_type = self
                        .module
                        .structs
                        .iter()
                        .any(|s| s.entity == parent_entity)
                        || self.module.enums.iter().any(|e| e.entity == parent_entity);
                    if !is_concrete_nongeneric_type {
                        continue;
                    }
                    let self_ty = MirTy::Named {
                        entity: parent_entity,
                        type_args: Vec::new(),
                    };
                    let inst = FunctionInstantiation {
                        func_id: FunctionId::new(i),
                        type_args: Vec::new(),
                        self_type: Some(self_ty),
                    };
                    if self.result.functions.insert(inst.clone()) {
                        self.queue.push_back(inst);
                    }
                }
            }
        }

        // BFS: process each instantiation
        while let Some(inst) = self.queue.pop_front() {
            self.process_instantiation(&inst);
        }
    }

    fn process_instantiation(&mut self, inst: &FunctionInstantiation) {
        self.processing_func_id = Some(inst.func_id);
        let func = &self.module.functions[inst.func_id.index()];

        // Boundary check: an instantiation reaching the monomorphizer must
        // have exactly as many type args as the function declares. A mismatch
        // here is a dispatch bug (some call site built a callee with wrong
        // arity); silently truncating via `build_subst`'s `zip` would produce
        // a subtly-wrong instantiation that fails cryptically later. Under-
        // specification is still legitimately skipped below in `scan_callee`
        // (phantom instantiations), so by the time we dequeue an inst, arity
        // must match.
        if func.type_params.len() != inst.type_args.len() {
            self.errors.push(MonomorphizeError::TypeArgArityMismatch {
                function: self.module.resolve_name(func.entity).to_string(),
                expected: func.type_params.len(),
                got: inst.type_args.len(),
            });
            return;
        }

        // Build substitution map for this instantiation, including associated types
        let mut subst = build_subst(func, &inst.type_args);
        crate::function::resolve_assoc_type_substs(
            self.module,
            func,
            &mut subst,
            inst.self_type.as_ref(),
        );

        // Scan function body for callees
        let Some(body) = &func.body else { return };

        for block in &body.blocks {
            for stmt in &block.stmts {
                match &stmt.kind {
                    StatementKind::Call {
                        callee, args: _, ..
                    } => {
                        self.scan_callee(callee, &subst, &inst.self_type);
                    },
                    StatementKind::Assign { rvalue, .. } => {
                        self.scan_rvalue(rvalue, &subst, &inst.self_type);
                    },
                    _ => {},
                }
            }

            // Scan terminator for values that might contain function refs
            // (Return and Branch contain Values that could be immediate function refs)
        }
    }

    fn scan_callee(
        &mut self,
        callee: &Callee,
        subst: &HashMap<Entity, MirTy>,
        parent_self: &Option<MirTy>,
    ) {
        match callee {
            Callee::Direct {
                func,
                type_args,
                self_type,
            } => {
                let Some(&func_id) = self.entity_to_func.get(func) else {
                    return;
                };

                // Substitute type args using the current instantiation's substitution
                let concrete_type_args =
                    common::substitute_type_args(type_args, subst, parent_self.as_ref());

                // Resolve self type: only inherit parent's self_type if the callee
                // actually uses SelfType in its signature. Static methods on other types
                // (e.g., Pointer[T].nullPointer() called from a TcpListener method)
                // should NOT inherit the caller's self_type.
                //
                // Closures/thunks are lexically nested in the parent's Self scope
                // even when their signatures don't syntactically reference
                // SelfType — their bodies can still mention protocol associated
                // types (e.g. `Iterator.Item`) that need the parent's self_type
                // to resolve via the witness table.
                let callee_func = &self.module.functions[func_id.index()];
                let callee_is_nested = matches!(
                    callee_func.kind,
                    FunctionKind::Closure
                        | FunctionKind::ClosureCall { .. }
                        | FunctionKind::Thunk { .. }
                );
                let concrete_self = self_type
                    .as_ref()
                    .map(|st| substitute_type_with_self(st, subst, parent_self.as_ref()))
                    .or_else(|| {
                        if self.func_uses_self_type(callee_func) || callee_is_nested {
                            parent_self.clone()
                        } else {
                            None
                        }
                    });

                // Skip phantom instantiations:
                // 1. Unresolved TypeParams in type_args or self_type
                // 2. Insufficient type_args for the callee's type_params (missing
                //    inherited struct type_args — correctly-resolved versions are
                //    discovered through call paths that propagate full type info)
                //
                // MirTy::Error is caught earlier by MIR-lower's validate pass
                // (which short-circuits `compile_inner`), so we don't re-check
                // for it here.
                if concrete_type_args.iter().any(|a| has_type_param(a)) {
                    return;
                }
                if let Some(ref st) = concrete_self {
                    if has_type_param(st) {
                        return;
                    }
                }
                if concrete_type_args.len() < callee_func.type_params.len() {
                    return;
                }

                let inst = FunctionInstantiation {
                    func_id,
                    type_args: concrete_type_args.clone(),
                    self_type: concrete_self,
                };

                if self.result.functions.insert(inst.clone()) {
                    self.queue.push_back(inst);
                }
            },

            Callee::Witness {
                protocol,
                method,
                self_type,
                method_type_args,
            } => {
                // Substitute type params AND SelfType using parent's self_type
                let mut concrete_self =
                    substitute_type_with_self(self_type, subst, parent_self.as_ref());
                // Resolve associated types (e.g., Iterator.Item → concrete type)
                // via witness table. Search all protocols for the owning one,
                // not just the protocol being called (handles cross-protocol
                // cases like Iterable.Iter used as self_type for Iterator.next)
                if let MirTy::Named {
                    entity,
                    ref type_args,
                } = concrete_self
                {
                    if type_args.is_empty() {
                        let assoc_name = self.module.resolve_name(entity);
                        let short = common::short_name(&assoc_name);

                        // Find which protocol owns this associated type
                        let owning_protocol = self
                            .module
                            .protocols
                            .iter()
                            .find(|p| p.associated_type_by_name(short).is_some())
                            .map(|p| p.entity);

                        if let Some(owner_proto) = owning_protocol {
                            // Try parent_self first, then search the subst map for a
                            // concrete type that implements the owning protocol
                            let candidates: Vec<&MirTy> = if let Some(ps) = parent_self {
                                vec![ps]
                            } else {
                                vec![]
                            };
                            // Also try all concrete types from the substitution map
                            let subst_values: Vec<&MirTy> = subst.values().collect();
                            for candidate in candidates.iter().chain(subst_values.iter()) {
                                if let Ok(resolved) = witness::resolve_associated_type(
                                    self.module,
                                    owner_proto,
                                    candidate,
                                    short,
                                ) {
                                    concrete_self = resolved;
                                    break;
                                }
                            }
                        }
                    }
                }
                let concrete_method_args: Vec<MirTy> = method_type_args
                    .iter()
                    .map(|a| substitute_type_with_self(a, subst, parent_self.as_ref()))
                    .collect();

                match witness::resolve_witness_call(
                    self.module,
                    *protocol,
                    method,
                    &concrete_self,
                    &concrete_method_args,
                ) {
                    Ok(resolved) => {
                        let Some(&func_id) = self.entity_to_func.get(&resolved.func_entity) else {
                            ktrace!(
                                "codegen",
                                "monomorphize: witness resolved to unknown entity {:?}",
                                resolved.func_entity
                            );
                            return;
                        };

                        if resolved.type_args.iter().any(|a| has_type_param(a)) {
                            return;
                        }

                        let inst = FunctionInstantiation {
                            func_id,
                            type_args: resolved.type_args,
                            self_type: resolved.self_type,
                        };

                        if self.result.functions.insert(inst.clone()) {
                            self.queue.push_back(inst);
                        }
                    },
                    Err(e) => {
                        self.errors.push(e);
                    },
                }
            },

            // Thin/Thick indirect calls don't introduce new instantiations
            Callee::Thin(_) | Callee::Thick(_) => {},
        }
    }

    fn scan_rvalue(
        &mut self,
        rvalue: &Rvalue,
        subst: &HashMap<Entity, MirTy>,
        parent_self: &Option<MirTy>,
    ) {
        match rvalue {
            // ApplyPartial introduces the target function as an instantiation
            Rvalue::ApplyPartial { func, captures: _ } => {
                if let Some((original_func_id, callable_func_id)) = self
                    .entity_to_func
                    .get(func)
                    .map(|id| (*id, self.apply_partial_callable_for(*func)))
                {
                    // Infer type args from the parent's substitution
                    let target = &self.module.functions[original_func_id.index()];
                    let type_args: Vec<MirTy> = target
                        .type_params
                        .iter()
                        .filter_map(|tp| subst.get(&tp.entity).cloned())
                        .collect();

                    // Always propagate parent's self_type: closures created
                    // inside an extension method inherit the enclosing Self
                    // scope, which is needed to resolve protocol associated
                    // types (e.g. `Item` in `extend Iterator where Item: ...`)
                    // against the concrete receiver via the witness table.
                    let inst = FunctionInstantiation {
                        func_id: callable_func_id,
                        type_args,
                        self_type: parent_self.clone(),
                    };

                    if self.result.functions.insert(inst.clone()) {
                        self.queue.push_back(inst);
                    }
                }
            },

            // Const with FunctionRef introduces the referenced function
            Rvalue::Const(imm) => {
                if let kestrel_mir::ImmediateKind::FunctionRef { func, type_args } = &imm.kind {
                    if let Some(&func_id) = self.entity_to_func.get(func) {
                        let concrete_type_args =
                            common::substitute_type_args(type_args, subst, parent_self.as_ref());

                        let inst = FunctionInstantiation {
                            func_id,
                            type_args: concrete_type_args,
                            self_type: parent_self.clone(),
                        };

                        if self.result.functions.insert(inst.clone()) {
                            self.queue.push_back(inst);
                        }
                    }
                }
            },

            _ => {},
        }
    }

    /// Check if a function's signature uses SelfType anywhere.
    fn func_uses_self_type(&self, func: &kestrel_mir::FunctionDef) -> bool {
        for param in &func.params {
            if type_uses_self(&param.ty) {
                return true;
            }
        }
        type_uses_self(&func.ret)
    }

    fn apply_partial_callable_for(&self, original: Entity) -> FunctionId {
        self.module
            .functions
            .iter()
            .enumerate()
            .find_map(|(i, func)| match &func.kind {
                FunctionKind::Thunk {
                    original: thunk_target,
                } if *thunk_target == original => Some(FunctionId::new(i)),
                _ => None,
            })
            .or_else(|| self.entity_to_func.get(&original).copied())
            .expect("ApplyPartial target must resolve to a function")
    }
}

/// Build a substitution map from a function's type params and concrete type args.
fn build_subst(func: &kestrel_mir::FunctionDef, type_args: &[MirTy]) -> HashMap<Entity, MirTy> {
    func.type_params
        .iter()
        .zip(type_args.iter())
        .map(|(tp, arg)| (tp.entity, arg.clone()))
        .collect()
}

/// Check if a type contains SelfType anywhere in its structure.
fn type_uses_self(ty: &MirTy) -> bool {
    match ty {
        MirTy::SelfType => true,
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => type_uses_self(inner),
        MirTy::Tuple(elems) => elems.iter().any(type_uses_self),
        MirTy::Named { type_args, .. } => type_args.iter().any(type_uses_self),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(type_uses_self) || type_uses_self(ret)
        },
        MirTy::AssociatedProjection { base, .. } => type_uses_self(base),
        _ => false,
    }
}

/// Check if a function's body references any TypeParam types that aren't
/// covered by the function's own type_params. This detects functions that
/// should be generic but aren't (missing inherited type params).
fn func_body_has_type_params(func: &kestrel_mir::FunctionDef) -> bool {
    let Some(body) = &func.body else { return false };
    // Check locals for TypeParam references
    for local in &body.locals {
        if has_type_param(&local.ty) {
            return true;
        }
    }
    // Check call type_args for TypeParam references
    for block in &body.blocks {
        for stmt in &block.stmts {
            if let kestrel_mir::StatementKind::Call { callee, .. } = &stmt.kind {
                match callee {
                    kestrel_mir::Callee::Direct { type_args, .. } => {
                        if type_args.iter().any(has_type_param) {
                            return true;
                        }
                    },
                    _ => {},
                }
            }
        }
    }
    false
}

/// Check if a type contains any unresolved TypeParam or Error.
fn has_type_param(ty: &MirTy) -> bool {
    match ty {
        MirTy::TypeParam(_) | MirTy::Error => true,
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => has_type_param(inner),
        MirTy::Tuple(elems) => elems.iter().any(has_type_param),
        MirTy::Named { type_args, .. } => type_args.iter().any(has_type_param),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(has_type_param) || has_type_param(ret)
        },
        MirTy::AssociatedProjection { base, .. } => has_type_param(base),
        _ => false,
    }
}

