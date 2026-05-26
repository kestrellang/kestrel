use std::collections::{HashMap, VecDeque};

use indexmap::{IndexMap, IndexSet};
use kestrel_hecs::Entity;

use crate::item::function::{FunctionDef, FunctionKind, WhereConstraint};
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

            // Lang intrinsics have no body — codegen handles them as ops
            if func.body.is_none() && func.extern_info.is_none() {
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
                source_entity: key.func_entity,
                span: None,
            });
            return;
        }

        // Build substitution map
        let subst = build_subst(func, &key.type_args, key.self_type, self.arena, self.protocols, self.witnesses);
        let parent_self = key.self_type;
        let caller_entity = key.func_entity;

        let Some(body) = &func.body else { return };

        for block in &body.blocks {
            for stmt in &block.stmts {
                let stmt_span = stmt.span.as_ref();
                match &stmt.kind {
                    StatementKind::Call { callee, args, .. } => {
                        self.scan_callee(callee, &subst, parent_self, caller_entity, stmt_span);
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
            self.scan_terminator_operands(&block.terminator, &subst, parent_self);
        }
    }

    fn scan_callee(
        &mut self,
        callee: &Callee,
        subst: &SubstMap,
        parent_self: Option<TyId>,
        caller_entity: Entity,
        stmt_span: Option<&kestrel_span::Span>,
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

                let callee_func = &self.functions[func_idx.index()];

                // Lang intrinsics: no body, no extern — codegen handles as ops
                if callee_func.body.is_none() && callee_func.extern_info.is_none() {
                    return;
                }

                let concrete_type_args: Vec<TyId> = type_args
                    .iter()
                    .map(|&ta| substitute_and_resolve(self.arena, self.witnesses, ta, subst))
                    .collect();
                let callee_is_nested = matches!(
                    callee_func.kind,
                    FunctionKind::Closure { .. }
                        | FunctionKind::ClosureCall { .. }
                        | FunctionKind::Thunk { .. }
                );
                let concrete_self = self_type
                    .map(|st| substitute_and_resolve(self.arena, self.witnesses, st, subst))
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
                if concrete_type_args.len() != callee_func.type_params.len() {
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
                let concrete_self = substitute_and_resolve(self.arena, self.witnesses, *self_type, subst);
                let concrete_method_args: Vec<TyId> = method_type_args
                    .iter()
                    .map(|&a| substitute_and_resolve(self.arena, self.witnesses, a, subst))
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
                        // Enrich with entity names and source location
                        let enriched = match &e {
                            MonoError::MethodNotFound { method, .. } => {
                                let self_desc = format_ty(self.arena, concrete_self, self.entity_names);
                                let proto_name = self.entity_names.get(protocol)
                                    .map(|s| s.as_str()).unwrap_or("?");
                                MonoError::MethodNotFound {
                                    protocol_name: proto_name.to_string(),
                                    method: method.clone(),
                                    type_description: self_desc,
                                    source_entity: caller_entity,
                                    span: stmt_span.cloned(),
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
                    .map(|&ta| substitute_and_resolve(self.arena, self.witnesses, ta, subst))
                    .collect();

                let concrete_self = self_type
                    .map(|st| substitute_and_resolve(self.arena, self.witnesses, st, subst))
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

    fn func_uses_self_type(&self, func: &FunctionDef) -> bool {
        // Protocol Self is TypeParam(protocol_entity). Detect it by checking
        // if the first param is a TypeParam not in the function's type_params list.
        let known_tps: std::collections::HashSet<Entity> =
            func.type_params.iter().map(|tp| tp.entity).collect();
        if let Some(first_param) = func.params.first() {
            if let MirTy::TypeParam(e) = self.arena.get(first_param.ty) {
                return !known_tps.contains(e);
            }
        }
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

// -- Shared helpers for SubstMap construction --

/// Build a complete SubstMap for a function instantiation.
///
/// Handles three sources of type bindings:
/// 1. Explicit type params from the instantiation key
/// 2. Implicit Self → concrete_type for protocol default methods and their closures
/// 3. Associated type projections resolved via witness lookup
///
/// After this returns, `substitute(arena, ty, &subst)` fully resolves all
/// TypeParams and AssociatedProjections in the function's types.
pub fn build_subst(
    func: &FunctionDef,
    type_args: &[TyId],
    self_type: Option<TyId>,
    arena: &mut TyArena,
    protocols: &[ProtocolDef],
    witnesses: &[WitnessDef],
) -> SubstMap {
    let mut subst = SubstMap::new();

    for (tp, &arg) in func.type_params.iter().zip(type_args.iter()) {
        subst.type_params.insert(tp.entity, arg);
    }

    if let Some(st) = self_type {
        if let Some(proto_entity) = detect_implicit_protocol(func, arena, protocols) {
            subst.type_params.entry(proto_entity).or_insert(st);
            populate_assoc_types(arena, witnesses, protocols, proto_entity, st, &mut subst);
        }
        // Protocol extension methods on concrete structs: the body may reference
        // TypeParam(protocol_entity) but detect_implicit_protocol only catches
        // protocol default methods. Map every protocol that self_type conforms to.
        for proto in protocols {
            if subst.type_params.contains_key(&proto.entity) {
                continue;
            }
            for wit in witnesses {
                if wit.protocol != proto.entity {
                    continue;
                }
                let mut bindings = HashMap::new();
                if witness::match_pattern(arena, wit.implementing_type, st, &mut bindings) {
                    subst.type_params.entry(proto.entity).or_insert(st);
                    populate_assoc_types(arena, witnesses, protocols, proto.entity, st, &mut subst);
                    break;
                }
            }
        }
    }

    // Where-clause enrichment: for each `T: Protocol` constraint, map the
    // protocol entity → concrete type, bind extension type params from witness
    // pattern matching, and populate associated types.
    if let Some(where_clause) = &func.where_clause {
        for constraint in &where_clause.constraints {
            if let WhereConstraint::Implements {
                type_param,
                protocol,
                protocol_type_args,
            } = constraint
            {
                let Some(concrete_ty) = subst.type_params.get(type_param).copied() else {
                    continue;
                };

                subst.type_params.entry(*protocol).or_insert(concrete_ty);

                for wit in witnesses.iter() {
                    if wit.protocol != *protocol {
                        continue;
                    }
                    let mut bindings = HashMap::new();
                    if !witness::match_pattern(arena, wit.implementing_type, concrete_ty, &mut bindings) {
                        continue;
                    }
                    for (pi, &wc_arg_entity) in protocol_type_args.iter().enumerate() {
                        if let Some(&proto_expr) = wit.proto_type_args.get(pi) {
                            if let MirTy::TypeParam(ext_entity) = arena.get(proto_expr) {
                                if !subst.type_params.contains_key(ext_entity) {
                                    if let Some(&cv) = subst.type_params.get(&wc_arg_entity) {
                                        subst.type_params.insert(*ext_entity, cv);
                                    }
                                }
                            }
                        }
                    }
                    for (entity, ty) in &bindings {
                        subst.type_params.entry(*entity).or_insert(*ty);
                    }
                    break;
                }

                populate_assoc_types(arena, witnesses, protocols, *protocol, concrete_ty, &mut subst);
            }
        }
    }

    subst
}

/// Detect whether a function has an implicit Self: Protocol constraint.
///
/// Returns the protocol entity if the first param is TypeParam(protocol_entity),
/// or (for closures/thunks only) if any param or return type references a
/// protocol's TypeParam.
pub fn detect_implicit_protocol(
    func: &FunctionDef,
    arena: &TyArena,
    protocols: &[ProtocolDef],
) -> Option<Entity> {
    if let Some(first_param) = func.params.first() {
        if let MirTy::TypeParam(entity) = arena.get(first_param.ty) {
            if protocols.iter().any(|p| p.entity == *entity) {
                return Some(*entity);
            }
        }
    }
    // Closures inside protocol default methods inherit self_type but their
    // first param is env pointer, not Self. Scan their types for protocol
    // TypeParams, gated by function kind to avoid scanning every method.
    if !matches!(
        func.kind,
        FunctionKind::Closure { .. }
            | FunctionKind::ClosureCall { .. }
            | FunctionKind::Thunk { .. }
    ) {
        return None;
    }
    for proto in protocols {
        if proto.associated_types.is_empty() {
            continue;
        }
        let used = func.params.iter().any(|p| references_type_param(arena, p.ty, proto.entity))
            || references_type_param(arena, func.ret, proto.entity);
        if used {
            return Some(proto.entity);
        }
    }
    None
}

/// Resolve all associated types for a (protocol, concrete_type) pair and
/// insert them into the SubstMap. Handles transitive chains like
/// `FilterIterator[I].Item = I.Item` via bounded witness lookup.
pub fn populate_assoc_types(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    protocols: &[ProtocolDef],
    protocol: Entity,
    concrete_ty: TyId,
    subst: &mut SubstMap,
) {
    let Some(proto_def) = protocols.iter().find(|p| p.entity == protocol) else {
        return;
    };
    for assoc in &proto_def.associated_types {
        let assoc_key = (concrete_ty, protocol, assoc.entity);
        if subst.assoc_types.contains_key(&assoc_key) {
            continue;
        }
        if let Some(bound_ty) = witness::resolve_associated_type(
            arena, witnesses, protocol, concrete_ty, assoc.entity,
        ) {
            let resolved = substitute(arena, bound_ty, subst);
            // Use deep_resolve for nested projections like
            // FlattenIterator.Item = Iterator.Item(Iterator.Item(I))
            let final_ty = deep_resolve(arena, witnesses, resolved, 0);
            subst.assoc_types.insert(assoc_key, final_ty);
        }
    }
}

/// Substitute a type through a SubstMap, then recursively resolve any
/// AssociatedProjection nodes that emerge. Use for callee type_args in
/// scan_callee where the parent's SubstMap resolves TypeParams but the
/// resulting type may contain projections that need witness lookup.
pub fn substitute_and_resolve(
    arena: &mut TyArena,
    witnesses: &[WitnessDef],
    ty: TyId,
    subst: &SubstMap,
) -> TyId {
    let sub = substitute(arena, ty, subst);
    deep_resolve(arena, witnesses, sub, 0)
}

/// Recursively walk a type tree and resolve all AssociatedProjection nodes
/// whose base is concrete via witness lookup.
fn deep_resolve(arena: &mut TyArena, witnesses: &[WitnessDef], ty: TyId, depth: u32) -> TyId {
    if depth > 16 {
        return ty;
    }
    match arena.get(ty).clone() {
        MirTy::AssociatedProjection {
            base,
            protocol,
            assoc_type,
        } => {
            let resolved_base = deep_resolve(arena, witnesses, base, depth + 1);
            if !has_type_param(arena, resolved_base) {
                if let Some(bound) = witness::resolve_associated_type(
                    arena, witnesses, protocol, resolved_base, assoc_type,
                ) {
                    return deep_resolve(arena, witnesses, bound, depth + 1);
                }
            }
            if resolved_base != base {
                arena.intern(MirTy::AssociatedProjection {
                    base: resolved_base,
                    protocol,
                    assoc_type,
                })
            } else {
                ty
            }
        }
        MirTy::Named { entity, type_args } => {
            let new_args: Vec<TyId> = type_args
                .iter()
                .map(|&a| deep_resolve(arena, witnesses, a, depth + 1))
                .collect();
            if new_args != type_args { arena.named(entity, new_args) } else { ty }
        }
        MirTy::Pointer(inner) => {
            let r = deep_resolve(arena, witnesses, inner, depth + 1);
            if r != inner { arena.pointer(r) } else { ty }
        }
        MirTy::Tuple(elems) => {
            let new: Vec<TyId> = elems.iter().map(|&e| deep_resolve(arena, witnesses, e, depth + 1)).collect();
            if new != elems { arena.tuple(new) } else { ty }
        }
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            let is_thin = matches!(arena.get(ty), MirTy::FuncThin { .. });
            let new_params: Vec<(TyId, crate::ty::ParamConvention)> = params
                .iter()
                .map(|&(p, c)| (deep_resolve(arena, witnesses, p, depth + 1), c))
                .collect();
            let new_ret = deep_resolve(arena, witnesses, ret, depth + 1);
            let changed = new_params.iter().zip(params.iter()).any(|((np, _), (op, _))| np != op)
                || new_ret != ret;
            if changed {
                if is_thin {
                    arena.intern(MirTy::FuncThin { params: new_params, ret: new_ret })
                } else {
                    arena.intern(MirTy::FuncThick { params: new_params, ret: new_ret })
                }
            } else {
                ty
            }
        }
        _ => ty,
    }
}

/// Check if a type tree contains TypeParam(entity) anywhere.
fn references_type_param(arena: &TyArena, ty: TyId, entity: Entity) -> bool {
    match arena.get(ty) {
        MirTy::TypeParam(e) => *e == entity,
        MirTy::Pointer(inner) => references_type_param(arena, *inner, entity),
        MirTy::Tuple(elems) => elems.iter().any(|&e| references_type_param(arena, e, entity)),
        MirTy::Named { type_args, .. } => type_args.iter().any(|&a| references_type_param(arena, a, entity)),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(|(p, _)| references_type_param(arena, *p, entity))
                || references_type_param(arena, *ret, entity)
        }
        MirTy::AssociatedProjection { base, .. } => references_type_param(arena, *base, entity),
        _ => false,
    }
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
            WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
            impl_func_entity,
            vec![],
        ));

        // The implementing function
        let impl_func = FunctionDef {
            entity: impl_func_entity,
            name: "Int64.isEqual".into(),
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
                    method: WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
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
