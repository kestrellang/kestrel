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
use kestrel_codegen2::{substitute_type, substitute_type_with_self};
use kestrel_debug::ktrace;
use kestrel_hecs::Entity;
use kestrel_mir::{
    Callee, FunctionId, FunctionKind, MirModule, MirTy, Place, Rvalue, StatementKind,
    TerminatorKind, Value,
};
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
        }
    }

    fn collect(&mut self) {
        // Seed: all non-generic, non-closure entry points
        for (i, func) in self.module.functions.iter().enumerate() {
            if !func.type_params.is_empty() {
                continue;
            }

            // Closures and thunks are always discovered through their parent
            // (ApplyPartial or FunctionRef), never seeded directly
            if matches!(
                func.kind,
                FunctionKind::ClosureCall { .. } | FunctionKind::Closure | FunctionKind::Thunk { .. }
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
                    FunctionKind::Initializer { parent } |
                    FunctionKind::Deinit { parent } |
                    FunctionKind::Method { parent, .. } |
                    FunctionKind::StaticMethod { parent } => Some(*parent),
                    _ => None,
                };
                if let Some(parent_entity) = parent {
                    // Only seed if parent is a concrete type (struct/enum).
                    // Protocol extension methods have parent = Extension entity
                    // (not a type) — they're discovered through witness calls
                    // with concrete self_types at call sites.
                    let is_concrete_type = self.module.structs.iter().any(|s| s.entity == parent_entity)
                        || self.module.enums.iter().any(|e| e.entity == parent_entity);
                    if !is_concrete_type {
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
        let func = &self.module.functions[inst.func_id.index()];
        // Build substitution map for this instantiation
        let subst = build_subst(func, &inst.type_args);

        // Scan function body for callees
        let Some(body) = &func.body else { return };

        for block in &body.blocks {
            for stmt in &block.stmts {
                match &stmt.kind {
                    StatementKind::Call {
                        callee, args: _, ..
                    } => {
                        self.scan_callee(callee, &subst, &inst.self_type);
                    }
                    StatementKind::Assign { rvalue, .. } => {
                        self.scan_rvalue(rvalue, &subst, &inst.self_type);
                    }
                    _ => {}
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
                let concrete_type_args: Vec<MirTy> =
                    type_args.iter().map(|a| substitute_type(a, subst)).collect();

                // Resolve self type: only inherit parent's self_type if the callee
                // actually uses SelfType in its signature. Static methods on other types
                // (e.g., Pointer[T].nullPointer() called from a TcpListener method)
                // should NOT inherit the caller's self_type.
                let callee_func = &self.module.functions[func_id.index()];
                let concrete_self = self_type
                    .as_ref()
                    .map(|st| substitute_type(st, subst))
                    .or_else(|| {
                        if self.func_uses_self_type(callee_func) {
                            parent_self.clone()
                        } else {
                            None
                        }
                    });

                let inst = FunctionInstantiation {
                    func_id,
                    type_args: concrete_type_args,
                    self_type: concrete_self,
                };

                if self.result.functions.insert(inst.clone()) {
                    self.queue.push_back(inst);
                }
            }

            Callee::Witness {
                protocol,
                method,
                self_type,
                method_type_args,
            } => {
                // Substitute type params AND SelfType using parent's self_type
                let mut concrete_self = substitute_type_with_self(
                    self_type, subst, parent_self.as_ref(),
                );
                // Resolve associated types (e.g., Iterator.Item → concrete type)
                // via witness table using the parent's self_type
                if let MirTy::Named { entity, ref type_args } = concrete_self {
                    if type_args.is_empty() {
                        if let Some(ps) = parent_self {
                            let assoc_name = self.module.resolve_name(entity);
                            let short_name = assoc_name.rsplit('.').next().unwrap_or(&assoc_name);
                            for proto_def in &self.module.protocols {
                                if proto_def.associated_types.iter().any(|at| at.name == short_name) {
                                    if let Ok(resolved) = witness::resolve_associated_type(
                                        self.module, proto_def.entity, ps, short_name,
                                    ) {
                                        concrete_self = resolved;
                                        break;
                                    }
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

                        let inst = FunctionInstantiation {
                            func_id,
                            type_args: resolved.type_args,
                            self_type: resolved.self_type,
                        };

                        if self.result.functions.insert(inst.clone()) {
                            self.queue.push_back(inst);
                        }
                    }
                    Err(e) => {
                        self.errors.push(e);
                    }
                }
            }

            // Thin/Thick indirect calls don't introduce new instantiations
            Callee::Thin(_) | Callee::Thick(_) => {}
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
                if let Some(&func_id) = self.entity_to_func.get(func) {
                    // Infer type args from the parent's substitution
                    let target = &self.module.functions[func_id.index()];
                    let type_args: Vec<MirTy> = target
                        .type_params
                        .iter()
                        .filter_map(|tp| subst.get(&tp.entity).cloned())
                        .collect();

                    let inst = FunctionInstantiation {
                        func_id,
                        type_args,
                        self_type: parent_self.clone(),
                    };

                    if self.result.functions.insert(inst.clone()) {
                        self.queue.push_back(inst);
                    }
                }
            }

            // Const with FunctionRef introduces the referenced function
            Rvalue::Const(imm) => {
                if let kestrel_mir::ImmediateKind::FunctionRef { func, type_args } = &imm.kind {
                    if let Some(&func_id) = self.entity_to_func.get(func) {
                        let concrete_type_args: Vec<MirTy> =
                            type_args.iter().map(|a| substitute_type(a, subst)).collect();

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
            }

            _ => {}
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
}

/// Build a substitution map from a function's type params and concrete type args.
fn build_subst(
    func: &kestrel_mir::FunctionDef,
    type_args: &[MirTy],
) -> HashMap<Entity, MirTy> {
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
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
            type_uses_self(inner)
        }
        MirTy::Tuple(elems) => elems.iter().any(type_uses_self),
        MirTy::Named { type_args, .. } => type_args.iter().any(type_uses_self),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(type_uses_self) || type_uses_self(ret)
        }
        MirTy::AssociatedProjection { base, .. } => type_uses_self(base),
        _ => false,
    }
}
