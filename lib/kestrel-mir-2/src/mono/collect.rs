use std::collections::{HashMap, VecDeque};

use indexmap::{IndexMap, IndexSet};
use kestrel_hecs::Entity;

use crate::item::function::{FunctionDef, FunctionKind};
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::item::enum_def::EnumDef;
use crate::item::witness::WitnessDef;
use crate::mono::types::InstantiationKey;
use crate::mono::witness::{self, MonoError};
use crate::statement::{Callee, Rvalue, StatementKind};
use crate::immediate::ImmediateKind;
use crate::operand::Operand;
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};
use crate::{FunctionIdx, TyId};

fn format_ty(arena: &TyArena, ty: TyId, names: &IndexMap<Entity, String>) -> String {
    match arena.get(ty) {
        MirTy::Named { entity, type_args } => {
            let name = names.get(entity).map(|s| s.as_str()).unwrap_or("?");
            if type_args.is_empty() {
                name.to_string()
            } else {
                let args: Vec<String> = type_args.iter().map(|&a| format_ty(arena, a, names)).collect();
                format!("{name}[{}]", args.join(", "))
            }
        }
        other => format!("{other:?}"),
    }
}

// -- Collection result --

pub struct CollectionResult {
    pub instantiations: IndexSet<InstantiationKey>,
    pub witness_cache: WitnessCache,
}

// -- Witness cache --

pub struct WitnessCache {
    pub resolved: HashMap<(Entity, TyId), WitnessCacheEntry>,
}

pub struct WitnessCacheEntry {
    pub witness_idx: usize,
    pub bindings: HashMap<Entity, TyId>,
}

impl WitnessCache {
    fn new() -> Self {
        Self {
            resolved: HashMap::new(),
        }
    }

    fn insert(&mut self, protocol: Entity, self_type: TyId, idx: usize, bindings: HashMap<Entity, TyId>) {
        self.resolved.insert((protocol, self_type), WitnessCacheEntry {
            witness_idx: idx,
            bindings,
        });
    }
}

// -- BFS collection --

pub fn collect_all(
    functions: &[FunctionDef],
    structs: &[StructDef],
    enums: &[EnumDef],
    protocols: &[ProtocolDef],
    witnesses: &[WitnessDef],
    arena: &mut TyArena,
    entity_names: &IndexMap<Entity, String>,
) -> Result<CollectionResult, Vec<MonoError>> {
    let mut ctx = CollectionContext::new(functions, structs, enums, protocols, witnesses, arena, entity_names);
    ctx.collect();

    if ctx.errors.is_empty() {
        Ok(CollectionResult {
            instantiations: ctx.seen,
            witness_cache: ctx.witness_cache,
        })
    } else {
        Err(ctx.errors)
    }
}

struct CollectionContext<'a> {
    functions: &'a [FunctionDef],
    structs: &'a [StructDef],
    enums: &'a [EnumDef],
    protocols: &'a [ProtocolDef],
    witnesses: &'a [WitnessDef],
    arena: &'a mut TyArena,
    entity_names: &'a IndexMap<Entity, String>,
    entity_to_func: HashMap<Entity, FunctionIdx>,
    queue: VecDeque<InstantiationKey>,
    seen: IndexSet<InstantiationKey>,
    witness_cache: WitnessCache,
    errors: Vec<MonoError>,
}

impl<'a> CollectionContext<'a> {
    fn new(
        functions: &'a [FunctionDef],
        structs: &'a [StructDef],
        enums: &'a [EnumDef],
        protocols: &'a [ProtocolDef],
        witnesses: &'a [WitnessDef],
        arena: &'a mut TyArena,
        entity_names: &'a IndexMap<Entity, String>,
    ) -> Self {
        let entity_to_func: HashMap<Entity, FunctionIdx> = functions
            .iter()
            .enumerate()
            .map(|(i, f)| (f.entity, FunctionIdx::new(i)))
            .collect();

        Self {
            functions,
            structs,
            enums,
            protocols,
            witnesses,
            arena,
            entity_names,
            entity_to_func,
            queue: VecDeque::new(),
            seen: IndexSet::new(),
            witness_cache: WitnessCache::new(),
            errors: Vec::new(),
        }
    }

    fn collect(&mut self) {
        // Seed: all non-generic, non-closure, non-thunk entry points
        for func in self.functions.iter() {
            if !func.type_params.is_empty() {
                continue;
            }

            // Closures and thunks are discovered through their parent
            if matches!(
                func.kind,
                FunctionKind::ClosureCall { .. }
                    | FunctionKind::Closure { .. }
                    | FunctionKind::Thunk { .. }
            ) {
                continue;
            }

            // Methods on non-concrete parents must be discovered via call sites
            let parent = match &func.kind {
                FunctionKind::Initializer { parent }
                | FunctionKind::Deinit { parent }
                | FunctionKind::Method { parent, .. }
                | FunctionKind::StaticMethod { parent } => Some(*parent),
                _ => None,
            };

            if let Some(parent_entity) = parent {
                let is_concrete_nongeneric = self
                    .structs
                    .iter()
                    .any(|s| s.entity == parent_entity && s.type_params.is_empty())
                    || self
                        .enums
                        .iter()
                        .any(|e| e.entity == parent_entity && e.type_params.is_empty());

                if !is_concrete_nongeneric {
                    continue;
                }

                // If the function uses SelfType, seed with explicit self_type
                if self.func_uses_self_type(func) {
                    let self_ty = self.arena.named(parent_entity, vec![]);
                    let key = InstantiationKey::new(func.entity, vec![], Some(self_ty));
                    if self.seen.insert(key.clone()) {
                        self.queue.push_back(key);
                    }
                    continue;
                }
            }

            let key = InstantiationKey::concrete(func.entity);
            if self.seen.insert(key.clone()) {
                self.queue.push_back(key);
            }
        }

        // BFS
        while let Some(key) = self.queue.pop_front() {
            self.process_instantiation(&key);
        }
    }

    fn process_instantiation(&mut self, key: &InstantiationKey) {
        let Some(&func_idx) = self.entity_to_func.get(&key.func_entity) else {
            return;
        };
        let func = &self.functions[func_idx.index()];

        // Arity check
        if func.type_params.len() != key.type_args.len() {
            self.errors.push(MonoError::TypeArgArityMismatch {
                function: func.name.clone(),
                expected: func.type_params.len(),
                got: key.type_args.len(),
            });
            return;
        }

        // Build substitution map
        let subst = build_subst(func, &key.type_args, key.self_type);
        let parent_self = key.self_type;

        let Some(body) = &func.body else { return };

        for block in &body.blocks {
            for stmt in &block.stmts {
                match &stmt.kind {
                    StatementKind::Call { callee, args, .. } => {
                        self.scan_callee(callee, &subst, parent_self);
                        // Scan args for function refs
                        for (op, _) in args {
                            self.scan_operand(op, &subst, parent_self);
                        }
                    }
                    StatementKind::Assign { rvalue, .. } => {
                        self.scan_rvalue(rvalue, &subst, parent_self);
                    }
                    _ => {}
                }
            }
            // Scan terminator operands for function refs
            self.scan_terminator_operands(&block.terminator, &subst, parent_self);
        }
    }

    fn scan_callee(
        &mut self,
        callee: &Callee,
        subst: &SubstMap,
        parent_self: Option<TyId>,
    ) {
        match callee {
            Callee::Direct {
                func,
                type_args,
                self_type,
            } => {
                let Some(&func_idx) = self.entity_to_func.get(func) else {
                    return;
                };

                // Substitute type args
                let concrete_type_args: Vec<TyId> = type_args
                    .iter()
                    .map(|&ta| substitute(self.arena, ta, subst))
                    .collect();

                // Resolve self_type
                let callee_func = &self.functions[func_idx.index()];
                let callee_is_nested = matches!(
                    callee_func.kind,
                    FunctionKind::Closure { .. }
                        | FunctionKind::ClosureCall { .. }
                        | FunctionKind::Thunk { .. }
                );
                let concrete_self = self_type
                    .map(|st| substitute(self.arena, st, subst))
                    .or_else(|| {
                        if self.func_uses_self_type(callee_func) || callee_is_nested {
                            parent_self
                        } else {
                            None
                        }
                    });

                // Skip phantom instantiations
                if concrete_type_args.iter().any(|&t| has_type_param(self.arena, t)) {
                    return;
                }
                if let Some(st) = concrete_self
                    && has_type_param(self.arena, st)
                {
                    return;
                }
                if concrete_type_args.len() < callee_func.type_params.len() {
                    return;
                }

                let key = InstantiationKey::new(*func, concrete_type_args, concrete_self);
                if self.seen.insert(key.clone()) {
                    self.queue.push_back(key);
                }
            }

            Callee::Witness {
                protocol,
                method,
                self_type,
                method_type_args,
            } => {
                let concrete_self = substitute(self.arena, *self_type, subst);
                let concrete_method_args: Vec<TyId> = method_type_args
                    .iter()
                    .map(|&a| substitute(self.arena, a, subst))
                    .collect();

                if has_type_param(self.arena, concrete_self) {
                    return;
                }

                match witness::resolve_witness_call(
                    self.arena,
                    self.witnesses,
                    self.protocols,
                    self.functions,
                    self.entity_names,
                    *protocol,
                    method,
                    concrete_self,
                    &concrete_method_args,
                ) {
                    Ok(resolved) => {
                        // Cache the witness resolution
                        if let Ok((widx, bindings)) = witness::find_witness_with_method(
                            self.arena,
                            self.witnesses,
                            self.protocols,
                            *protocol,
                            method,
                            concrete_self,
                        ) {
                            self.witness_cache.insert(*protocol, concrete_self, widx, bindings);
                        }

                        let Some(&_func_idx) = self.entity_to_func.get(&resolved.func_entity)
                        else {
                            return;
                        };

                        if resolved.type_args.iter().any(|&t| has_type_param(self.arena, t)) {
                            return;
                        }

                        let key = InstantiationKey::new(
                            resolved.func_entity,
                            resolved.type_args,
                            resolved.self_type,
                        );
                        if self.seen.insert(key.clone()) {
                            self.queue.push_back(key);
                        }
                    }
                    Err(e) => {
                        // Enrich with entity names for diagnostics
                        let enriched = match &e {
                            MonoError::MethodNotFound { method, .. } => {
                                let self_desc = format_ty(self.arena, concrete_self, self.entity_names);
                                let proto_name = self.entity_names.get(protocol)
                                    .map(|s| s.as_str()).unwrap_or("?");
                                MonoError::MethodNotFound {
                                    protocol_name: proto_name.to_string(),
                                    method: method.clone(),
                                    type_description: self_desc,
                                }
                            }
                            _ => e,
                        };
                        self.errors.push(enriched);
                    }
                }
            }

            Callee::Thin(_) | Callee::Thick(_) | Callee::Resolved(_) => {}
        }
    }

    fn scan_rvalue(
        &mut self,
        rvalue: &Rvalue,
        subst: &SubstMap,
        parent_self: Option<TyId>,
    ) {
        match rvalue {
            Rvalue::ApplyPartial { func, captures: _ } => {
                if let Some(callable_idx) = self.apply_partial_callable_for(*func) {
                    let target = &self.functions[callable_idx.index()];
                    let type_args: Vec<TyId> = target
                        .type_params
                        .iter()
                        .filter_map(|tp| subst.type_params.get(&tp.entity).copied())
                        .collect();

                    let key = InstantiationKey::new(
                        self.functions[callable_idx.index()].entity,
                        type_args,
                        parent_self,
                    );
                    if self.seen.insert(key.clone()) {
                        self.queue.push_back(key);
                    }
                }
            }

            Rvalue::Use(operand, _) => {
                self.scan_operand(operand, subst, parent_self);
            }

            _ => {}
        }
    }

    fn scan_operand(
        &mut self,
        operand: &Operand,
        subst: &SubstMap,
        parent_self: Option<TyId>,
    ) {
        if let Operand::Const(imm) = operand
            && let ImmediateKind::FunctionRef {
                func,
                type_args,
                self_type,
            } = &imm.kind
        {
            let Some(&func_idx) = self.entity_to_func.get(func) else {
                return;
            };

                let concrete_type_args: Vec<TyId> = type_args
                    .iter()
                    .map(|&ta| substitute(self.arena, ta, subst))
                    .collect();

                let concrete_self = self_type
                    .map(|st| substitute(self.arena, st, subst))
                    .or(parent_self);

                let key = InstantiationKey::new(
                    *func,
                    concrete_type_args,
                    concrete_self,
                );
                if self.seen.insert(key.clone()) {
                    self.queue.push_back(key);
                }
                let _ = func_idx;
        }
    }

    fn scan_terminator_operands(
        &mut self,
        terminator: &crate::terminator::Terminator,
        subst: &SubstMap,
        parent_self: Option<TyId>,
    ) {
        use crate::terminator::TerminatorKind;
        match &terminator.kind {
            TerminatorKind::Return(op) => self.scan_operand(op, subst, parent_self),
            TerminatorKind::Branch { condition, .. } => {
                self.scan_operand(condition, subst, parent_self);
            }
            _ => {}
        }
    }

    fn func_uses_self_type(&self, _func: &FunctionDef) -> bool {
        // SelfType was eliminated during MIR lowering — Self is now lowered as
        // Named(parent_entity, [TypeParam...]), so no function uses SelfType.
        false
    }

    fn apply_partial_callable_for(&self, original: Entity) -> Option<FunctionIdx> {
        // First look for a thunk wrapping this function
        self.functions
            .iter()
            .enumerate()
            .find_map(|(i, func)| match &func.kind {
                FunctionKind::Thunk {
                    original: thunk_target,
                } if *thunk_target == original => Some(FunctionIdx::new(i)),
                _ => None,
            })
            .or_else(|| self.entity_to_func.get(&original).copied())
    }
}

// -- Helpers --

fn build_subst(func: &FunctionDef, type_args: &[TyId], _self_type: Option<TyId>) -> SubstMap {
    let mut subst = SubstMap::new();
    for (tp, &arg) in func.type_params.iter().zip(type_args.iter()) {
        subst.type_params.insert(tp.entity, arg);
    }
    subst
}

/// Check if a type contains any unresolved TypeParam.
pub fn has_type_param(arena: &TyArena, ty: TyId) -> bool {
    match arena.get(ty) {
        MirTy::TypeParam(_) | MirTy::Error => true,
        MirTy::Pointer(inner) => has_type_param(arena, *inner),
        MirTy::Tuple(elems) => elems.iter().any(|&e| has_type_param(arena, e)),
        MirTy::Named { type_args, .. } => type_args.iter().any(|&a| has_type_param(arena, a)),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(|(p, _)| has_type_param(arena, *p)) || has_type_param(arena, *ret)
        }
        MirTy::AssociatedProjection { base, .. } => has_type_param(arena, *base),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BasicBlock, LocalDef, MirBody};
    use crate::item::function::{FunctionDef, FunctionKind};
    use crate::item::struct_def::StructDef;
    use crate::item::witness::{WitnessDef, WitnessMethodBinding};
    use crate::item::TypeParamDef;
    use crate::statement::{Statement, WitnessMethodKey};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::{BlockId, LocalId};

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn make_body(stmts: Vec<Statement>, ret_local: LocalId, locals: Vec<LocalDef>) -> MirBody {
        let block = BasicBlock {
            stmts,
            terminator: Terminator {
                kind: TerminatorKind::Return(Operand::Place(crate::place::Place::local(ret_local))),
                span: None,
            },
        };
        MirBody {
            locals,
            blocks: vec![block],
            param_count: 0,
            entry: BlockId::new(0),
            local_scopes: HashMap::new(),
            failure_return_blocks: std::collections::HashSet::new(),
        }
    }

    // -- has_type_param --

    #[test]
    fn has_type_param_primitive() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        assert!(!has_type_param(&a, i64));
    }

    #[test]
    fn has_type_param_param() {
        let mut a = TyArena::new();
        let tp = a.intern(MirTy::TypeParam(entity(1)));
        assert!(has_type_param(&a, tp));
    }

    #[test]
    fn has_type_param_nested() {
        let mut a = TyArena::new();
        let tp = a.intern(MirTy::TypeParam(entity(1)));
        let ptr = a.pointer(tp);
        assert!(has_type_param(&a, ptr));
    }

    #[test]
    fn has_type_param_concrete_named() {
        let mut a = TyArena::new();
        let i64 = a.i64();
        let named = a.named(entity(1), vec![i64]);
        assert!(!has_type_param(&a, named));
    }

    // -- collect_all --

    #[test]
    fn collect_non_generic_function() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let func = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let result = collect_all(
            &[func], &[], &[], &[], &[], &mut arena, &IndexMap::new(),
        ).unwrap();

        assert_eq!(result.instantiations.len(), 1);
        assert!(result.instantiations.contains(&InstantiationKey::concrete(entity(1))));
    }

    #[test]
    fn collect_skips_generic_function() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let func = FunctionDef {
            entity: entity(1),
            name: "generic".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(2), "T")],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let result = collect_all(
            &[func], &[], &[], &[], &[], &mut arena, &IndexMap::new(),
        ).unwrap();

        assert!(result.instantiations.is_empty());
    }

    #[test]
    fn collect_discovers_direct_callee() {
        let mut arena = TyArena::new();
        let unit = arena.unit();
        let i64 = arena.i64();

        // generic_fn[T] — has one type param
        let generic_fn = FunctionDef {
            entity: entity(2),
            name: "generic_fn".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(3), "T")],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        // main() calls generic_fn[Int64]
        let call_stmt = Statement {
            kind: StatementKind::Call {
                dest: None,
                callee: Callee::Direct {
                    func: entity(2),
                    type_args: vec![i64],
                    self_type: None,
                },
                args: vec![],
            },
            span: None,
        };

        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![call_stmt], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let result = collect_all(
            &[main_fn, generic_fn],
            &[], &[], &[], &[],
            &mut arena,
            &IndexMap::new(),
        ).unwrap();

        assert_eq!(result.instantiations.len(), 2);
        // main (concrete) + generic_fn[Int64]
        assert!(result.instantiations.contains(&InstantiationKey::concrete(entity(1))));
        assert!(result.instantiations.contains(&InstantiationKey::new(
            entity(2),
            vec![i64],
            None,
        )));
    }

    #[test]
    fn collect_skips_closures_and_thunks() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let closure = FunctionDef {
            entity: entity(1),
            name: "closure".into(),
            kind: FunctionKind::Closure { parent_func: entity(2) },
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let thunk = FunctionDef {
            entity: entity(3),
            name: "thunk".into(),
            kind: FunctionKind::Thunk { original: entity(1) },
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let result = collect_all(
            &[closure, thunk], &[], &[], &[], &[], &mut arena, &IndexMap::new(),
        ).unwrap();

        assert!(result.instantiations.is_empty());
    }

    #[test]
    fn collect_method_on_concrete_parent() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let struct_def = StructDef::new(entity(1), "Point");

        let method = FunctionDef {
            entity: entity(2),
            name: "Point.x".into(),
            kind: FunctionKind::Method {
                parent: entity(1),
                receiver: crate::item::function::ReceiverConvention::Borrow,
            },
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let result = collect_all(
            &[method], &[struct_def], &[], &[], &[], &mut arena, &IndexMap::new(),
        ).unwrap();

        // Method on concrete non-generic parent is seeded
        assert_eq!(result.instantiations.len(), 1);
    }

    #[test]
    fn collect_witness_call_resolves() {
        let mut arena = TyArena::new();
        let unit = arena.unit();
        let i64 = arena.i64();

        let proto = entity(10);
        let impl_func_entity = entity(20);

        // Protocol
        let protocol = ProtocolDef::new(proto, "Equatable");

        // Witness: Int64: Equatable
        let mut witness = WitnessDef::new(proto, i64);
        witness.add_method(WitnessMethodBinding::new(
            WitnessMethodKey::simple("equals"),
            impl_func_entity,
            vec![],
        ));

        // The implementing function
        let impl_func = FunctionDef {
            entity: impl_func_entity,
            name: "Int64.equals".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        // main() has a witness call
        let call_stmt = Statement {
            kind: StatementKind::Call {
                dest: None,
                callee: Callee::Witness {
                    protocol: proto,
                    method: WitnessMethodKey::simple("equals"),
                    self_type: i64,
                    method_type_args: vec![],
                },
                args: vec![],
            },
            span: None,
        };

        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![call_stmt], LocalId::new(0), vec![
                LocalDef { name: "_ret".into(), ty: unit },
            ])),
            extern_info: None,
        };

        let mut names = IndexMap::new();
        names.insert(proto, "Equatable".to_string());

        let result = collect_all(
            &[main_fn, impl_func],
            &[], &[],
            &[protocol],
            &[witness],
            &mut arena,
            &names,
        ).unwrap();

        // main + Int64.equals
        assert_eq!(result.instantiations.len(), 2);
        assert!(result.instantiations.contains(&InstantiationKey::concrete(impl_func_entity)));
    }
}
