use std::collections::{HashMap, VecDeque};

use indexmap::{IndexMap, IndexSet};
use kestrel_hecs::Entity;

use crate::TyId;
use crate::callee::Callee;
use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::item::enum_def::EnumDef;
use crate::item::function::{FunctionDef, FunctionKind, WhereConstraint};
use crate::item::protocol::ProtocolDef;
use crate::item::struct_def::StructDef;
use crate::item::witness::WitnessDef;
use crate::mono::types::InstantiationKey;
use crate::mono::witness::{self, MonoError};
use crate::substitute::{SubstMap, substitute};
use crate::ty::{MirTy, TyArena};

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

    fn insert(
        &mut self,
        protocol: Entity,
        self_type: TyId,
        idx: usize,
        bindings: HashMap<Entity, TyId>,
    ) {
        self.resolved.insert(
            (protocol, self_type),
            WitnessCacheEntry {
                witness_idx: idx,
                bindings,
            },
        );
    }
}

// -- BFS collection --

pub fn collect_all(
    functions: &IndexMap<Entity, FunctionDef>,
    structs: &IndexMap<Entity, StructDef>,
    enums: &IndexMap<Entity, EnumDef>,
    protocols: &IndexMap<Entity, ProtocolDef>,
    witnesses: &[WitnessDef],
    arena: &mut TyArena,
    entity_names: &IndexMap<Entity, String>,
) -> Result<CollectionResult, Vec<MonoError>> {
    let mut ctx = CollectionContext::new(
        functions,
        structs,
        enums,
        protocols,
        witnesses,
        arena,
        entity_names,
    );
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
    functions: &'a IndexMap<Entity, FunctionDef>,
    structs: &'a IndexMap<Entity, StructDef>,
    enums: &'a IndexMap<Entity, EnumDef>,
    protocols: &'a IndexMap<Entity, ProtocolDef>,
    witnesses: &'a [WitnessDef],
    arena: &'a mut TyArena,
    entity_names: &'a IndexMap<Entity, String>,
    queue: VecDeque<InstantiationKey>,
    seen: IndexSet<InstantiationKey>,
    witness_cache: WitnessCache,
    errors: Vec<MonoError>,
}

impl<'a> CollectionContext<'a> {
    fn new(
        functions: &'a IndexMap<Entity, FunctionDef>,
        structs: &'a IndexMap<Entity, StructDef>,
        enums: &'a IndexMap<Entity, EnumDef>,
        protocols: &'a IndexMap<Entity, ProtocolDef>,
        witnesses: &'a [WitnessDef],
        arena: &'a mut TyArena,
        entity_names: &'a IndexMap<Entity, String>,
    ) -> Self {
        Self {
            functions,
            structs,
            enums,
            protocols,
            witnesses,
            arena,
            entity_names,
            queue: VecDeque::new(),
            seen: IndexSet::new(),
            witness_cache: WitnessCache::new(),
            errors: Vec::new(),
        }
    }

    fn collect(&mut self) {
        // Seed: all non-generic, non-closure, non-thunk entry points
        for func in self.functions.values() {
            if !func.type_params.is_empty() {
                continue;
            }

            // Lang intrinsics have no body — codegen handles them as ops
            if func.body.is_none() && func.extern_info.is_none() {
                continue;
            }

            // Closures, thunks, and drop shims are discovered through callers.
            // Drop shims must be discovered via DestroyValue scanning so they
            // inherit the correct self_type (protocol context for associated types).
            if matches!(
                func.kind,
                FunctionKind::ClosureCall { .. }
                    | FunctionKind::Closure { .. }
                    | FunctionKind::Thunk { .. }
                    | FunctionKind::DropShim { .. }
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
                    .get(&parent_entity)
                    .is_some_and(|s| s.type_params.is_empty())
                    || self
                        .enums
                        .get(&parent_entity)
                        .is_some_and(|e| e.type_params.is_empty());

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
        let Some(func) = self.functions.get(&key.func_entity) else {
            return;
        };

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
        let subst = build_subst(
            func,
            &key.type_args,
            key.self_type,
            self.arena,
            self.protocols,
            self.witnesses,
        );
        let parent_self = key.self_type;
        let caller_entity = key.func_entity;

        let Some(body) = &func.body else { return };

        for block in &body.blocks {
            for inst in &block.insts {
                match &inst.kind {
                    InstKind::Call { callee, .. } => {
                        self.scan_callee(
                            callee,
                            &subst,
                            parent_self,
                            caller_entity,
                            inst.span.as_ref(),
                        );
                    },
                    InstKind::Literal { value, .. } => {
                        self.scan_immediate(&value.kind, &subst, parent_self);
                    },
                    InstKind::ApplyPartial { callee, .. } => {
                        // The partial-applied closure/thunk is referenced exactly
                        // like a Call callee (a `Callee::Direct` after the thunk
                        // pass), so discover its instantiation the same way — this
                        // is what binds each `read[T]` to its own thunk instead of
                        // collapsing to the first.
                        self.scan_callee(
                            callee,
                            &subst,
                            parent_self,
                            caller_entity,
                            inst.span.as_ref(),
                        );
                    },
                    InstKind::DestroyValue { operand } => {
                        let operand_ty = body.values[operand.index()].ty;
                        let concrete_ty =
                            substitute_and_resolve(self.arena, self.witnesses, operand_ty, &subst);
                        self.discover_drop_shim(concrete_ty, parent_self);
                    },
                    InstKind::DestroyAddr { ty, .. } => {
                        let concrete_ty =
                            substitute_and_resolve(self.arena, self.witnesses, *ty, &subst);
                        self.discover_drop_shim(concrete_ty, parent_self);
                    },
                    InstKind::StoreAssign { address, .. } => {
                        // The address is Pointer[T]; discover drop shim for T.
                        let addr_ty = body.values[address.index()].ty;
                        let concrete_addr =
                            substitute_and_resolve(self.arena, self.witnesses, addr_ty, &subst);
                        if let MirTy::Pointer(pointee) = self.arena.get(concrete_addr) {
                            self.discover_drop_shim(*pointee, parent_self);
                        }
                    },
                    InstKind::CopyValue { operand, .. } => {
                        let operand_ty = body.values[operand.index()].ty;
                        let concrete_ty =
                            substitute_and_resolve(self.arena, self.witnesses, operand_ty, &subst);
                        self.discover_clone_shim(concrete_ty, parent_self);
                    },
                    _ => {},
                }
            }
            // No terminator scanning needed — MIR terminators carry ValueId, not Operand
        }
    }

    fn scan_callee(
        &mut self,
        callee: &Callee,
        subst: &SubstMap,
        parent_self: Option<TyId>,
        _caller_entity: Entity,
        _stmt_span: Option<&kestrel_span::Span>,
    ) {
        match callee {
            Callee::Direct {
                func,
                type_args,
                self_type,
            } => {
                let Some(callee_func) = self.functions.get(func) else {
                    return;
                };

                // Lang intrinsics: no body, no extern — codegen handles as ops
                if callee_func.body.is_none() && callee_func.extern_info.is_none() {
                    return;
                }

                let mut concrete_type_args: Vec<TyId> = type_args
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

                // Normalize arity to the callee's own type-param count *before*
                // the phantom check. Context inference can over-provide args
                // drawn from a wrapper result type (`Result[T,E]`/`Optional[T]`
                // of a throwing init or Optional-returning factory) even for a
                // non-generic callee; drop the excess so the key matches the
                // arity `rewrite_callee` looks up (which truncates identically).
                // A genuine under-count can't be instantiated, so skip it.
                if !normalize_direct_arity(&mut concrete_type_args, callee_func.type_params.len()) {
                    return;
                }

                // Skip phantom instantiations
                if concrete_type_args
                    .iter()
                    .any(|&t| has_type_param(self.arena, t))
                {
                    return;
                }
                if let Some(st) = concrete_self
                    && has_type_param(self.arena, st)
                {
                    return;
                }

                let key = InstantiationKey::new(*func, concrete_type_args, concrete_self);
                if self.seen.insert(key.clone()) {
                    self.queue.push_back(key);
                }
            },

            Callee::Witness {
                protocol,
                method,
                self_type,
                method_type_args,
            } => {
                let concrete_self =
                    substitute_and_resolve(self.arena, self.witnesses, *self_type, subst);
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
                            &concrete_method_args,
                        ) {
                            self.witness_cache
                                .insert(*protocol, concrete_self, widx, bindings);
                        }

                        if !self.functions.contains_key(&resolved.func_entity) {
                            return;
                        }

                        if resolved
                            .type_args
                            .iter()
                            .any(|&t| has_type_param(self.arena, t))
                        {
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
                    },
                    Err(e) => {
                        // MethodNotFound for witness dispatch is non-fatal:
                        // the function simply won't be instantiated, and
                        // codegen will emit a trap stub if it's ever called.
                        if !matches!(&e, MonoError::MethodNotFound { .. }) {
                            self.errors.push(e);
                        }
                    },
                }
            },

            Callee::Thin(_) | Callee::Thick(_) | Callee::Resolved(_) => {},
        }
    }

    /// Scan an ImmediateKind for FunctionRef — discovers function references
    /// used as values (closures, first-class function pointers).
    fn scan_immediate(&mut self, imm: &ImmediateKind, subst: &SubstMap, parent_self: Option<TyId>) {
        if let ImmediateKind::FunctionRef {
            func,
            type_args,
            self_type,
        } = imm
        {
            if !self.functions.contains_key(func) {
                return;
            }

            let concrete_type_args: Vec<TyId> = type_args
                .iter()
                .map(|&ta| substitute_and_resolve(self.arena, self.witnesses, ta, subst))
                .collect();

            let concrete_self = self_type
                .map(|st| substitute_and_resolve(self.arena, self.witnesses, st, subst))
                .or(parent_self);

            let key = InstantiationKey::new(*func, concrete_type_args, concrete_self);
            if self.seen.insert(key.clone()) {
                self.queue.push_back(key);
            }
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

    /// If `ty` is a Named type with a drop shim, enqueue the shim instantiation.
    fn discover_drop_shim(&mut self, ty: TyId, parent_self: Option<TyId>) {
        match self.arena.get(ty) {
            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                let type_args = type_args.clone();
                if let Some(shim) = self.functions.values().find(
                    |f| matches!(f.kind, FunctionKind::DropShim { nominal } if nominal == entity),
                ) {
                    let key = InstantiationKey::new(shim.entity, type_args, parent_self);
                    if self.seen.insert(key.clone()) {
                        self.queue.push_back(key);
                    }
                }
            },
            // A tuple has no nominal entity (no `__drop$tuple`), so its members'
            // shims must be discovered through it — otherwise a resource type
            // used *only* inside a tuple never gets its drop shim instantiated.
            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                for e in elems {
                    self.discover_drop_shim(e, parent_self);
                }
            },
            _ => {},
        }
    }

    /// If `ty` is a Named type with a clone shim, enqueue the shim instantiation.
    /// Tuples are recursed into so their members' clone shims are discovered.
    fn discover_clone_shim(&mut self, ty: TyId, parent_self: Option<TyId>) {
        match self.arena.get(ty) {
            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                let type_args = type_args.clone();
                // Find clone shim or user clone method
                let clone_func = self
                    .functions
                    .values()
                    .find(
                        |f| matches!(f.kind, FunctionKind::CloneShim { nominal } if nominal == entity),
                    )
                    .or_else(|| {
                        // Match a user `clone()` by its self-param nominal: an
                        // `extend`-defined clone doesn't reliably set `parent` to
                        // the extended type, so `parent == entity` would miss it
                        // (leaving the clone uncollected and the value bit-copied).
                        self.functions
                            .values()
                            .find(|f| f.clone_method_self_nominal(self.arena) == Some(entity))
                    });
                if let Some(func) = clone_func {
                    let key = InstantiationKey::new(func.entity, type_args, parent_self);
                    if self.seen.insert(key.clone()) {
                        self.queue.push_back(key);
                    }
                }
            },
            // A tuple has no nominal entity, so its members' clone shims must be
            // discovered through it — otherwise a resource type used *only*
            // inside a tuple is bit-copied (the shim never gets instantiated).
            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                for e in elems {
                    self.discover_clone_shim(e, parent_self);
                }
            },
            _ => {},
        }
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
    protocols: &IndexMap<Entity, ProtocolDef>,
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
        for proto in protocols.values() {
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
                    if !witness::match_pattern(
                        arena,
                        wit.implementing_type,
                        concrete_ty,
                        &mut bindings,
                    ) {
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

                populate_assoc_types(
                    arena,
                    witnesses,
                    protocols,
                    *protocol,
                    concrete_ty,
                    &mut subst,
                );
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
    protocols: &IndexMap<Entity, ProtocolDef>,
) -> Option<Entity> {
    if let Some(first_param) = func.params.first() {
        if let MirTy::TypeParam(entity) = arena.get(first_param.ty) {
            if protocols.contains_key(entity) {
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
    for proto in protocols.values() {
        if proto.associated_types.is_empty() {
            continue;
        }
        let used = func
            .params
            .iter()
            .any(|p| references_type_param(arena, p.ty, proto.entity))
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
    protocols: &IndexMap<Entity, ProtocolDef>,
    protocol: Entity,
    concrete_ty: TyId,
    subst: &mut SubstMap,
) {
    let Some(proto_def) = protocols.get(&protocol) else {
        return;
    };
    for assoc in &proto_def.associated_types {
        let assoc_key = (concrete_ty, protocol, assoc.entity);
        if subst.assoc_types.contains_key(&assoc_key) {
            continue;
        }
        if let Some(bound_ty) =
            witness::resolve_associated_type(arena, witnesses, protocol, concrete_ty, assoc.entity)
        {
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
                    arena,
                    witnesses,
                    protocol,
                    resolved_base,
                    assoc_type,
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
        },
        MirTy::Named { entity, type_args } => {
            let new_args: Vec<TyId> = type_args
                .iter()
                .map(|&a| deep_resolve(arena, witnesses, a, depth + 1))
                .collect();
            if new_args != type_args {
                arena.named(entity, new_args)
            } else {
                ty
            }
        },
        MirTy::Pointer(inner) => {
            let r = deep_resolve(arena, witnesses, inner, depth + 1);
            if r != inner { arena.pointer(r) } else { ty }
        },
        MirTy::Tuple(elems) => {
            let new: Vec<TyId> = elems
                .iter()
                .map(|&e| deep_resolve(arena, witnesses, e, depth + 1))
                .collect();
            if new != elems { arena.tuple(new) } else { ty }
        },
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            let is_thin = matches!(arena.get(ty), MirTy::FuncThin { .. });
            let new_params: Vec<(TyId, crate::ty::ParamConvention)> = params
                .iter()
                .map(|&(p, c)| (deep_resolve(arena, witnesses, p, depth + 1), c))
                .collect();
            let new_ret = deep_resolve(arena, witnesses, ret, depth + 1);
            let changed = new_params
                .iter()
                .zip(params.iter())
                .any(|((np, _), (op, _))| np != op)
                || new_ret != ret;
            if changed {
                if is_thin {
                    arena.intern(MirTy::FuncThin {
                        params: new_params,
                        ret: new_ret,
                    })
                } else {
                    arena.intern(MirTy::FuncThick {
                        params: new_params,
                        ret: new_ret,
                    })
                }
            } else {
                ty
            }
        },
        _ => ty,
    }
}

/// Check if a type tree contains TypeParam(entity) anywhere.
fn references_type_param(arena: &TyArena, ty: TyId, entity: Entity) -> bool {
    match arena.get(ty) {
        MirTy::TypeParam(e) => *e == entity,
        MirTy::Pointer(inner) => references_type_param(arena, *inner, entity),
        MirTy::Tuple(elems) => elems
            .iter()
            .any(|&e| references_type_param(arena, e, entity)),
        MirTy::Named { type_args, .. } => type_args
            .iter()
            .any(|&a| references_type_param(arena, a, entity)),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params
                .iter()
                .any(|(p, _)| references_type_param(arena, *p, entity))
                || references_type_param(arena, *ret, entity)
        },
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
        },
        MirTy::AssociatedProjection { base, .. } => has_type_param(arena, *base),
        _ => false,
    }
}

/// Normalize a `Callee::Direct`'s type-arg list to the callee's own type-param
/// count, in place. Returns `false` (caller should skip) when the list is
/// *shorter* than the arity — a genuinely uninstantiable call. Excess args
/// (over-provided by context inference from a wrapper result type — e.g.
/// `Result[T,E]`/`Optional[T]` of a throwing init or Optional-returning
/// factory whose callee is non-generic) are truncated so the instantiation key
/// matches between collection and `rewrite_callee`. Single source of truth for
/// the Direct-arity invariant.
pub(crate) fn normalize_direct_arity(type_args: &mut Vec<TyId>, type_param_count: usize) -> bool {
    use std::cmp::Ordering;
    match type_args.len().cmp(&type_param_count) {
        Ordering::Greater => {
            type_args.truncate(type_param_count);
            true
        },
        Ordering::Less => false,
        Ordering::Equal => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BasicBlock;
    use crate::body::OssaBody;
    use crate::callee::Callee;
    use crate::inst::{InstKind, Instruction};
    use crate::item::TypeParamDef;
    use crate::item::function::{FunctionDef, FunctionKind};
    use crate::item::protocol::ProtocolDef;
    use crate::item::struct_def::StructDef;
    use crate::item::witness::{WitnessDef, WitnessMethodBinding};
    use crate::terminator::{Terminator, TerminatorKind};
    use crate::value::ValueDef;

    fn func_map(funcs: Vec<FunctionDef>) -> IndexMap<Entity, FunctionDef> {
        funcs.into_iter().map(|f| (f.entity, f)).collect()
    }
    fn struct_map(structs: Vec<StructDef>) -> IndexMap<Entity, StructDef> {
        structs.into_iter().map(|s| (s.entity, s)).collect()
    }
    fn proto_map(protos: Vec<ProtocolDef>) -> IndexMap<Entity, ProtocolDef> {
        protos.into_iter().map(|p| (p.entity, p)).collect()
    }
    use crate::ValueId;
    use crate::item::WitnessMethodKey;

    fn entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    /// Build a single-block OssaBody with the given instructions, returning `ret_val`.
    fn make_body(insts: Vec<Instruction>, ret_val: ValueId, values: Vec<ValueDef>) -> OssaBody {
        let mut block = BasicBlock::new();
        block.insts = insts;
        block.terminator = Terminator::new(TerminatorKind::Return(ret_val));
        OssaBody {
            values,
            blocks: vec![block],
            entry: crate::BlockId::new(0),
            param_count: 0,
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

        let ret_val = ValueId::new(0);
        let func = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        let result = collect_all(
            &func_map(vec![func]),
            &struct_map(vec![]),
            &IndexMap::new(),
            &proto_map(vec![]),
            &[],
            &mut arena,
            &IndexMap::new(),
        )
        .unwrap();

        assert_eq!(result.instantiations.len(), 1);
        assert!(
            result
                .instantiations
                .contains(&InstantiationKey::concrete(entity(1)))
        );
    }

    #[test]
    fn collect_skips_generic_function() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let ret_val = ValueId::new(0);
        let func = FunctionDef {
            entity: entity(1),
            name: "generic".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(2), "T")],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        let result = collect_all(
            &func_map(vec![func]),
            &struct_map(vec![]),
            &IndexMap::new(),
            &proto_map(vec![]),
            &[],
            &mut arena,
            &IndexMap::new(),
        )
        .unwrap();

        assert!(result.instantiations.is_empty());
    }

    #[test]
    fn collect_discovers_direct_callee() {
        let mut arena = TyArena::new();
        let unit = arena.unit();
        let i64 = arena.i64();

        // generic_fn[T] — has one type param
        let gen_ret_val = ValueId::new(0);
        let generic_fn = FunctionDef {
            entity: entity(2),
            name: "generic_fn".into(),
            kind: FunctionKind::Free,
            type_params: vec![TypeParamDef::new(entity(3), "T")],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], gen_ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        // main() calls generic_fn[Int64]
        let call_inst = Instruction::new(InstKind::Call {
            result: None,
            callee: Callee::Direct {
                func: entity(2),
                type_args: vec![i64],
                self_type: None,
            },
            args: vec![],
        });

        let main_ret_val = ValueId::new(0);
        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![call_inst],
                main_ret_val,
                vec![ValueDef::owned(unit)],
            )),
            extern_info: None,
        };

        let result = collect_all(
            &func_map(vec![main_fn, generic_fn]),
            &struct_map(vec![]),
            &IndexMap::new(),
            &proto_map(vec![]),
            &[],
            &mut arena,
            &IndexMap::new(),
        )
        .unwrap();

        assert_eq!(result.instantiations.len(), 2);
        // main (concrete) + generic_fn[Int64]
        assert!(
            result
                .instantiations
                .contains(&InstantiationKey::concrete(entity(1)))
        );
        assert!(
            result
                .instantiations
                .contains(&InstantiationKey::new(entity(2), vec![i64], None,))
        );
    }

    #[test]
    fn collect_skips_closures_and_thunks() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let ret_val = ValueId::new(0);
        let closure = FunctionDef {
            entity: entity(1),
            name: "closure".into(),
            kind: FunctionKind::Closure {
                parent_func: entity(2),
            },
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        let thunk = FunctionDef {
            entity: entity(3),
            name: "thunk".into(),
            kind: FunctionKind::Thunk {
                original: entity(1),
            },
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        let result = collect_all(
            &func_map(vec![closure, thunk]),
            &struct_map(vec![]),
            &IndexMap::new(),
            &proto_map(vec![]),
            &[],
            &mut arena,
            &IndexMap::new(),
        )
        .unwrap();

        assert!(result.instantiations.is_empty());
    }

    #[test]
    fn collect_method_on_concrete_parent() {
        let mut arena = TyArena::new();
        let unit = arena.unit();

        let struct_def = StructDef::new(entity(1), "Point");

        let ret_val = ValueId::new(0);
        let method = FunctionDef {
            entity: entity(2),
            name: "Point.x".into(),
            kind: FunctionKind::Method {
                parent: entity(1),
                receiver: crate::ty::ParamConvention::Borrow,
            },
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        let result = collect_all(
            &func_map(vec![method]),
            &struct_map(vec![struct_def]),
            &IndexMap::new(),
            &proto_map(vec![]),
            &[],
            &mut arena,
            &IndexMap::new(),
        )
        .unwrap();

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
        let impl_ret_val = ValueId::new(0);
        let impl_func = FunctionDef {
            entity: impl_func_entity,
            name: "Int64.isEqual".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(vec![], impl_ret_val, vec![ValueDef::owned(unit)])),
            extern_info: None,
        };

        // main() has a witness call
        let call_inst = Instruction::new(InstKind::Call {
            result: None,
            callee: Callee::Witness {
                protocol: proto,
                method: WitnessMethodKey::new("isEqual", vec![Some("to".into())]),
                self_type: i64,
                method_type_args: vec![],
            },
            args: vec![],
        });

        let main_ret_val = ValueId::new(0);
        let main_fn = FunctionDef {
            entity: entity(1),
            name: "main".into(),
            kind: FunctionKind::Free,
            type_params: vec![],
            params: vec![],
            ret: unit,
            where_clause: None,
            body: Some(make_body(
                vec![call_inst],
                main_ret_val,
                vec![ValueDef::owned(unit)],
            )),
            extern_info: None,
        };

        let mut names = IndexMap::new();
        names.insert(proto, "Equatable".to_string());

        let result = collect_all(
            &func_map(vec![main_fn, impl_func]),
            &struct_map(vec![]),
            &IndexMap::new(),
            &proto_map(vec![protocol]),
            &[witness],
            &mut arena,
            &names,
        )
        .unwrap();

        // main + Int64.equals
        assert_eq!(result.instantiations.len(), 2);
        assert!(
            result
                .instantiations
                .contains(&InstantiationKey::concrete(impl_func_entity))
        );
    }
}
