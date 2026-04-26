//! Type resolver trait and world implementation.
//!
//! `TypeResolver` abstracts world queries for testability. The solver
//! depends on this trait, not on concrete `QueryContext`. `WorldResolver`
//! is the real implementation; tests can provide mocks.

use kestrel_ast_builder::{
    AstType, Callable, ConformanceItem, Conformances, Gettable, Name, NodeKind, Settable, Static,
    TypeParams, Vis, WhereClause as AstWhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{
    LowerCallableReturnType, LowerCallableTypes, LowerExtensionTargetTypeArgs, LowerTypeAnnotation,
};
use kestrel_name_res::{
    ConformingProtocols, ResolveBuiltin, ResolveTypePath, TypeResolution,
    expand_protocol_closure_in_place,
};
use kestrel_span::Span;

use crate::ty::TyKind;

/// Result of resolving a member on a type.
#[derive(Clone, Debug)]
pub struct MemberResolution {
    /// The resolved entity (function, field, getter, etc.)
    pub entity: Entity,
    /// Type parameters of the member (to be instantiated with fresh TyVars).
    pub type_params: Vec<Entity>,
    /// Parameter types (with type param placeholders as `HirTy::Param`).
    pub param_types: Vec<ParamInfo>,
    /// Return type (with type param placeholders).
    pub return_type: HirTy,
    /// Where clauses on this member.
    pub where_clauses: Vec<WhereClause>,
    /// What kind of member this is.
    pub kind: MemberKind,
    /// The entity that `Self` resolves to in the member's scope.
    /// For protocol extension methods, this is the protocol entity.
    /// The solver substitutes this with the actual receiver type.
    pub self_type: Option<Entity>,
    /// Set when resolved through a protocol conformance rather than directly.
    /// The solver emits a Conforms constraint to validate the receiver
    /// conforms to this protocol with the inferred type args.
    pub via_protocol: Option<Entity>,
    /// Type arguments applied to the protocol in the where clause (e.g., `[lang.i64]`
    /// for `F: Factory[lang.i64]`). Used to substitute protocol type params
    /// in the method's return type and parameter types.
    pub protocol_type_args: Vec<HirTy>,
    /// The extension entity this member was resolved from (if any).
    /// Used by the solver to check type arg compatibility and where clause satisfaction.
    pub from_extension: Option<Entity>,
}

/// Info about a member's parameter, for overload resolution.
#[derive(Clone, Debug)]
pub struct ParamInfo {
    pub label: Option<String>,
    pub ty: HirTy,
    pub has_default: bool,
}

/// What kind of member was resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberKind {
    Field { mutable: bool },
    Method,
    ComputedProperty { has_setter: bool },
    Subscript,
    Init,
}

/// Error from member resolution.
#[derive(Clone, Debug)]
pub enum MemberError {
    NotFound,
    /// Ambiguous candidates, ranked by extension specificity (most specific first).
    /// Each entry: (candidate_entity, from_extension, specificity).
    /// Direct members have specificity 0. Higher specificity = more concrete
    /// extension type args. The solver picks uniquely-top-specificity candidates
    /// after filtering by compatibility; ties at the top become AmbiguousMember.
    Ambiguous(Vec<(Entity, Option<Entity>, usize)>),
    /// Member exists but is not visible from the requesting context. Carries
    /// the offending entity and its declared visibility so the diagnostic can
    /// report which access modifier blocks the access.
    NotVisible {
        candidate: Entity,
        visibility: Vis,
    },
}

/// Result of resolving an associated type.
#[derive(Clone, Debug)]
pub struct AssociatedTypeResolution {
    /// The concrete type this associated type resolves to.
    pub resolved: HirTy,
    /// The extension that supplied this binding, if any.
    /// Set when an `extend ConcreteType: Proto[FreeParams]` block introduces
    /// the binding — `solve_associated` reads this to substitute the
    /// extension's free TypeParams with the call-site's protocol args.
    /// `None` when resolved directly off the type, off a protocol's abstract
    /// declaration, or off where-clause bounds.
    pub source_extension: Option<Entity>,
}

/// Where clause on a declaration.
#[derive(Clone, Debug, Hash)]
pub enum WhereClause {
    /// `T: Protocol` or `T: Protocol[Args]`
    Bound {
        param: Entity,
        protocol: Entity,
        /// Type arguments applied to the protocol (e.g., `[lang.i64]` in `Factory[lang.i64]`).
        /// Empty for non-generic protocols.
        protocol_type_args: Vec<HirTy>,
    },
    /// `T.Item = SomeType` (associated type equality)
    TypeEquality {
        param: Entity,
        assoc_name: String,
        rhs: HirTy,
    },
    /// `V = Array[E]` (direct type parameter equality)
    DirectEquality { param: Entity, rhs: HirTy },
}

/// Slim trait abstracting world queries for testability.
pub trait TypeResolver {
    /// Resolve a member on a type by name and arguments.
    fn resolve_member(
        &self,
        receiver_ty: &TyKind,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError>;

    /// Resolve a static member on a type (e.g., Result.fromResidual).
    /// Like resolve_member but searches static methods instead of instance ones.
    fn resolve_static_member(
        &self,
        receiver_ty: &TyKind,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        let _ = (receiver_ty, name, args);
        Err(MemberError::NotFound)
    }

    /// Check if a concrete type conforms to a protocol.
    fn conforms_to(&self, ty: &TyKind, protocol: Entity) -> bool;

    /// Resolve an associated type on a container (e.g., Array[Int].Element → Int).
    fn resolve_associated_type(
        &self,
        container: &TyKind,
        name: &str,
    ) -> Option<AssociatedTypeResolution>;

    /// Look up a builtin entity by language feature.
    fn builtin(&self, feature: Builtin) -> Option<Entity>;

    // Note: where clauses are not exposed on this trait. Callers use
    // `crate::where_clauses::WhereClausesOf { entity, root }` directly —
    // that query resolves names in the entity's own scope, avoiding the
    // "ambient owner" leak that motivated this design.

    /// Build a MemberResolution for a specific known member entity.
    /// Used by the solver when picking from ranked ambiguous candidates.
    fn resolve_single_member(
        &self,
        receiver_ty: &TyKind,
        member: Entity,
    ) -> Result<MemberResolution, MemberError>;

    /// Check if `to` type can be constructed from `from` type via FromValue promotion.
    /// Returns the `from()` method entity if promotion is possible.
    fn check_promotion(&self, from: &TyKind, to: &TyKind) -> Option<Entity>;
}

// ===== WorldResolver: real implementation over QueryContext =====

/// Implements TypeResolver using the ECS world.
///
/// `body_owner` is the body being inferred — the name-resolution context for
/// things like member lookups and variable references. It is deliberately
/// named with the `body_` prefix so it's obvious at the call site that any
/// reference to it ties the lookup to "wherever this method call is written",
/// not to a type-declaration scope. Type-declaration-scoped resolution (e.g.
/// where clauses) must NOT use `body_owner` — use the `WhereClausesOf` query
/// (or pass the target entity explicitly) so resolution is stable regardless
/// of which body triggered it.
pub struct WorldResolver<'a> {
    pub ctx: &'a QueryContext<'a>,
    pub root: Entity,
    pub body_owner: Entity,
}

impl TypeResolver for WorldResolver<'_> {
    fn resolve_member(
        &self,
        receiver_ty: &TyKind,
        name: &str,
        _args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        // Type parameters: search protocol bounds for the member
        if let TyKind::Param { entity } = receiver_ty {
            return self.resolve_param_member(*entity, name, _args);
        }

        // Abstract associated-type projection: member-lookup through the
        // assoc entity's bounds (same machinery as TypeAlias below).
        if let TyKind::AssocProjection { assoc, .. } = receiver_ty {
            match self.resolve_assoc_type_member(*assoc, name, _args) {
                Ok(res) => return Ok(res),
                Err(MemberError::NotFound) => {
                    return self.resolve_assoc_type_static_member_resolve(*assoc, name, _args);
                },
                Err(e) => return Err(e),
            }
        }

        // TypeAlias receiver: abstract associated types consult protocol bounds;
        // concrete aliases should have been reduced by the solver's Reduce rule
        // before reaching here (but handle defensively in case they show up).
        if let TyKind::TypeAlias { entity, .. } = receiver_ty {
            match self.resolve_assoc_type_member(*entity, name, _args) {
                Ok(res) => return Ok(res),
                Err(MemberError::NotFound) => {
                    return self.resolve_assoc_type_static_member_resolve(*entity, name, _args);
                },
                Err(e) => return Err(e),
            }
        }

        let Some(entity) = receiver_ty.entity() else {
            return Err(MemberError::NotFound);
        };
        let entity = &entity;

        // Search direct children by name
        let candidates = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *entity,
            name: name.to_string(),
            context: self.body_owner,
        });

        // Also search extensions of the concrete type
        let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
            target: *entity,
            root: self.root,
        });
        let mut all_candidates = candidates;
        // Track which extension each candidate came from (for solver-side filtering)
        let mut candidate_extensions: Vec<(Entity, Entity)> = Vec::new(); // (candidate, extension)
        for ext in &extensions {
            let ext_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                parent: *ext,
                name: name.to_string(),
                context: self.body_owner,
            });
            for &child in &ext_children {
                candidate_extensions.push((child, *ext));
            }
            all_candidates.extend(ext_children);
        }

        // Fallback: search protocol extensions for default method implementations.
        // E.g., `lessThan` lives in `extend Comparable { ... }`, not in the protocol
        // itself. We only look at extension-provided members (not the abstract
        // requirements themselves) to avoid ambiguity with the default impl.
        //
        // Multiple protocol extensions may provide the same method (e.g. notEquals
        // from both extend Equatable and extend Comparable). These are equivalent
        // default implementations, so we deduplicate by label signature.
        if all_candidates.is_empty() {
            let protocols = self.ctx.query(kestrel_name_res::ConformingProtocols {
                entity: *entity,
                root: self.root,
            });
            let mut seen_signatures = std::collections::HashSet::new();
            for proto in &protocols {
                let members = self.ctx.query(kestrel_name_res::ProtocolMembersByName {
                    protocol: *proto,
                    name: name.to_string(),
                    context: self.body_owner,
                    root: self.root,
                });
                for m in members {
                    let Some(ext) = m.extension else { continue };
                    let sig = self.label_signature(m.entity);
                    if seen_signatures.insert(sig) {
                        all_candidates.push(m.entity);
                        candidate_extensions.push((m.entity, ext));
                    }
                }
            }
        }

        // If receiver is a protocol and we're inside a protocol extension with
        // where clauses like `Self: Sortable`, also search those constraint protocols.
        // `ProtocolMembersByName` walks the constraint protocol's direct children,
        // extension defaults, parent protocols, and parents' extension defaults in
        // one pass — so a `where Self: Comparable` extension can resolve methods
        // declared on `Equatable` (Comparable's parent).
        if all_candidates.is_empty()
            && self.ctx.get::<NodeKind>(*entity) == Some(&NodeKind::Protocol)
        {
            let extra_protocols = self.collect_extension_where_clause_protocols(*entity);
            for proto in &extra_protocols {
                let members = self.ctx.query(kestrel_name_res::ProtocolMembersByName {
                    protocol: *proto,
                    name: name.to_string(),
                    context: self.body_owner,
                    root: self.root,
                });
                all_candidates.extend(members.into_iter().map(|m| m.entity));
            }
        }

        // Initializers have no Name component, so VisibleChildrenByName won't
        // find them. Search by NodeKind::Initializer when looking for "init".
        if all_candidates.is_empty() && name == "init" {
            for &child in self.ctx.children_of(*entity) {
                if self.ctx.get::<NodeKind>(child) == Some(&NodeKind::Initializer) {
                    all_candidates.push(child);
                }
            }
            // Also search extensions (e.g., extend Array: ExpressibleByArrayLiteral { init(...) })
            for ext in &extensions {
                for &child in self.ctx.children_of(*ext) {
                    if self.ctx.get::<NodeKind>(child) == Some(&NodeKind::Initializer) {
                        all_candidates.push(child);
                    }
                }
            }
        }

        // Subscripts have no Name component (like initializers).
        // Search by NodeKind::Subscript when resolving a subscript call.
        if all_candidates.is_empty() && name == "subscript" {
            for &child in self.ctx.children_of(*entity) {
                if self.ctx.get::<NodeKind>(child) == Some(&NodeKind::Subscript) {
                    all_candidates.push(child);
                }
            }
            for ext in &extensions {
                for &child in self.ctx.children_of(*ext) {
                    if self.ctx.get::<NodeKind>(child) == Some(&NodeKind::Subscript) {
                        all_candidates.push(child);
                    }
                }
            }
        }

        // Filter out static members (we're resolving on an instance)
        let instance_candidates: Vec<Entity> = all_candidates
            .into_iter()
            .filter(|&c| !self.ctx.has::<Static>(c))
            .collect();

        if instance_candidates.is_empty() {
            // One final probe: a matching member that was filtered out by
            // visibility? If so, report NotVisible so diagnostics can say
            // "is private" rather than "no member".
            if let Some((candidate, visibility)) =
                self.find_hidden_member(*entity, name, &extensions)
            {
                return Err(MemberError::NotVisible {
                    candidate,
                    visibility,
                });
            }
            return Err(MemberError::NotFound);
        }

        // Overload resolution: filter by matching argument labels and arity
        let arg_labels: Vec<Option<&str>> = _args.iter().map(|a| a.label.as_deref()).collect();
        let matches: Vec<Entity> = instance_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, &arg_labels))
            .collect();

        let member = match matches.len() {
            0 => {
                // No label match — fall back to single candidate ONLY if its arity
                // accepts the call. If arity is wrong too, treat as NotFound so the
                // diagnostic is "no member" rather than a misleading "wrong arity".
                if instance_candidates.len() == 1
                    && self.matches_arity(instance_candidates[0], arg_labels.len())
                {
                    instance_candidates[0]
                } else {
                    return Err(MemberError::NotFound);
                }
            },
            1 => matches[0],
            _ => {
                // Multiple candidates with same labels — try protocol-based resolution.
                // This is only valid when every concrete match is an implementation
                // of the same protocol requirement. Unrelated overloads sharing a
                // label signature (e.g., Array's internal `_arrayLiteralPointer:`
                // protocol init alongside its separate `count:generator:` overload)
                // must fall through to genuine ambiguity, not be silently resolved
                // to the protocol's method.
                if let Some(proto_res) =
                    self.try_resolve_through_protocol(*entity, name, _args, &matches)
                {
                    return Ok(proto_res);
                }

                // Rank by extension specificity: more concrete type args = more specific.
                // Return candidates sorted by specificity for solver-side filtering.
                let ranked = self.rank_by_extension_specificity(&matches, &candidate_extensions);
                return Err(MemberError::Ambiguous(ranked));
            },
        };

        let mut resolution = self.build_member_resolution(member)?;
        // Track which extension the member came from (for solver-side type arg filtering)
        if let Some(&(_, ext)) = candidate_extensions.iter().find(|(c, _)| *c == member) {
            resolution.from_extension = Some(ext);
        }
        Ok(resolution)
    }

    fn resolve_single_member(
        &self,
        _receiver_ty: &TyKind,
        member: Entity,
    ) -> Result<MemberResolution, MemberError> {
        self.build_member_resolution(member)
    }

    fn conforms_to(&self, ty: &TyKind, protocol: Entity) -> bool {
        match ty {
            TyKind::Struct { entity, .. }
            | TyKind::Enum { entity, .. }
            | TyKind::Protocol { entity, .. } => {
                let all_protocols = self.ctx.query(kestrel_name_res::ConformingProtocols {
                    entity: *entity,
                    root: self.root,
                });
                all_protocols.contains(&protocol)
            },
            TyKind::SelfType { entity } => {
                // Abstract Self of protocol P conforms to P (and P's parents).
                // Use the same conforming-protocols walk as for Protocol.
                let all_protocols = self.ctx.query(kestrel_name_res::ConformingProtocols {
                    entity: *entity,
                    root: self.root,
                });
                all_protocols.contains(&protocol)
            },
            TyKind::TypeAlias { entity, .. } => {
                // Associated-type bounds (e.g. `type Item: Equatable`) live on
                // the TypeAlias entity itself.
                let bound_protocols = self.collect_assoc_type_protocol_bounds(*entity);
                bound_protocols.contains(&protocol)
            },
            TyKind::Param { entity } => {
                let bound_protocols = self.collect_param_protocol_bounds(*entity);
                bound_protocols.contains(&protocol)
            },
            TyKind::AssocProjection { assoc, .. } => {
                // Conformance bounds on the associated type.
                let bound_protocols = self.collect_assoc_type_protocol_bounds(*assoc);
                bound_protocols.contains(&protocol)
            },
            _ => false,
        }
    }

    fn resolve_associated_type(
        &self,
        container: &TyKind,
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        match container {
            TyKind::Struct { entity, .. } | TyKind::Enum { entity, .. } => {
                // Concrete type — search children for a TypeAlias with matching name,
                // then extensions (e.g. Dictionary's `type Key = K` lives on an
                // `extend Dictionary[K, V, H]: _ExpressibleByDictionaryLiteral` block).
                if let Some(res) = self.find_associated_type_in_entity(*entity, name) {
                    return Some(res);
                }
                let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                    target: *entity,
                    root: self.root,
                });
                for ext in &extensions {
                    if let Some(mut res) = self.find_associated_type_in_entity(*ext, name) {
                        // Record the source extension so solve_associated can
                        // substitute the extension's free TypeParams with the
                        // call-site's protocol args.
                        res.source_extension = Some(*ext);
                        return Some(res);
                    }
                }
                None
            },
            TyKind::Protocol { entity, .. } | TyKind::SelfType { entity } => {
                self.find_associated_type_in_protocol(*entity, name)
            },
            TyKind::TypeAlias { entity, .. } => {
                // Protocol associated type (e.g. Iter: Iterator) —
                // search the bound protocols for the name.
                let bound_protocols = self.collect_assoc_type_protocol_bounds(*entity);
                self.find_associated_type_in_protocols(&bound_protocols, name)
            },
            TyKind::Param { entity } => {
                let bound_protocols = self.collect_param_protocol_bounds(*entity);
                self.find_associated_type_in_protocols(&bound_protocols, name)
            },
            TyKind::AssocProjection { assoc, .. } => {
                // Nested: T.Iter.Item — search Iter's bound protocols for Item.
                let bound_protocols = self.collect_assoc_type_protocol_bounds(*assoc);
                self.find_associated_type_in_protocols(&bound_protocols, name)
            },
            _ => None,
        }
    }

    fn builtin(&self, feature: Builtin) -> Option<Entity> {
        self.ctx.query(ResolveBuiltin {
            builtin: feature,
            root: self.root,
        })
    }

    fn check_promotion(&self, _from: &TyKind, to: &TyKind) -> Option<Entity> {
        let to_entity = &match to {
            TyKind::Struct { entity, .. }
            | TyKind::Enum { entity, .. }
            | TyKind::Protocol { entity, .. } => *entity,
            _ => return None,
        };

        // Resolve the FromValue protocol
        let from_value_protocol = self.builtin(Builtin::FromValueProtocol)?;

        // Check if the target type conforms to FromValue
        if !self.conforms_to(to, from_value_protocol) {
            return None;
        }

        // Find the static `from` method on the target type (via extensions)
        let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
            target: *to_entity,
            root: self.root,
        });

        for ext in &extensions {
            // Check if this extension provides FromValue conformance
            if !self.extension_directly_conforms_to(*ext, from_value_protocol) {
                continue;
            }

            // Look for a static `from` method in this extension
            let children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                parent: *ext,
                name: "from".to_string(),
                context: self.body_owner,
            });

            for &child in &children {
                if self.ctx.has::<Static>(child)
                    && matches!(self.ctx.get::<NodeKind>(child), Some(NodeKind::Function))
                {
                    return Some(child);
                }
            }
        }

        // Also check direct children (in case FromValue is implemented directly)
        let direct = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *to_entity,
            name: "from".to_string(),
            context: self.body_owner,
        });

        for &child in &direct {
            if self.ctx.has::<Static>(child)
                && matches!(self.ctx.get::<NodeKind>(child), Some(NodeKind::Function))
            {
                return Some(child);
            }
        }

        None
    }

    fn resolve_static_member(
        &self,
        receiver_ty: &TyKind,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        let Some(entity_val) = receiver_ty.entity() else {
            return Err(MemberError::NotFound);
        };
        let entity = &entity_val;

        // Search direct children and extensions for static members
        let mut all_candidates: Vec<kestrel_hecs::Entity> = Vec::new();

        let children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *entity,
            name: name.to_string(),
            context: self.body_owner,
        });
        all_candidates.extend(children.iter());

        let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
            target: *entity,
            root: self.root,
        });
        for ext in &extensions {
            let ext_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                parent: *ext,
                name: name.to_string(),
                context: self.body_owner,
            });
            all_candidates.extend(ext_children.iter());
        }

        // Filter to static members only
        let static_candidates: Vec<kestrel_hecs::Entity> = all_candidates
            .into_iter()
            .filter(|&c| self.ctx.has::<Static>(c))
            .collect();

        if std::env::var("DEBUG_STATIC_MEMBER").is_ok() {
            eprintln!(
                "resolve_static_member: name={} candidates={}",
                name,
                static_candidates.len()
            );
            for &c in &static_candidates {
                let callable = self.ctx.get::<kestrel_ast_builder::Callable>(c);
                eprintln!(
                    "  candidate {:?} params={:?}",
                    c,
                    callable.map(|cc| cc
                        .params
                        .iter()
                        .map(|p| (p.label.clone(), p.name.clone()))
                        .collect::<Vec<_>>())
                );
            }
        }

        if static_candidates.is_empty() {
            return Err(MemberError::NotFound);
        }

        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let matches: Vec<kestrel_hecs::Entity> = static_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, &arg_labels))
            .collect();

        if std::env::var("DEBUG_STATIC_MEMBER").is_ok() {
            eprintln!("  arg_labels={:?} matches={}", arg_labels, matches.len());
        }

        let member = match matches.len() {
            0 => {
                if static_candidates.len() == 1
                    && self.matches_arity(static_candidates[0], arg_labels.len())
                {
                    static_candidates[0]
                } else {
                    return Err(MemberError::NotFound);
                }
            },
            1 => matches[0],
            _ => {
                return Err(MemberError::Ambiguous(
                    matches.into_iter().map(|e| (e, None, 0)).collect(),
                ));
            },
        };

        self.build_member_resolution(member)
    }
}

/// Compatibility check between a protocol method's parameter types and a
/// concrete candidate's parameter types. Type params, `Self`, `Infer`, and
/// `Error` act as wildcards (they can stand for anything). Concrete
/// constructors must match at every non-wildcard position.
///
/// This is intentionally conservative: it rejects clear mismatches (e.g.
/// `lang.ptr[Element]` vs `Int64`, or `lang.i64` vs `(Int64) -> T`) so that
/// unrelated overloads sharing a label signature don't collapse into a
/// protocol requirement they don't actually implement. Full unification
/// isn't needed here — we just need to keep genuinely-incompatible overloads
/// out of the protocol-abstract representation.
fn param_shapes_compatible(proto: &[HirTy], concrete: &[HirTy]) -> bool {
    if proto.len() != concrete.len() {
        return false;
    }
    proto
        .iter()
        .zip(concrete.iter())
        .all(|(p, c)| ty_shape_compatible(p, c))
}

fn ty_shape_compatible(a: &HirTy, b: &HirTy) -> bool {
    // Wildcards — a protocol param or a recovery/infer slot matches anything.
    if is_shape_wildcard(a) || is_shape_wildcard(b) {
        return true;
    }
    match (a, b) {
        (
            HirTy::Struct {
                entity: ea,
                args: aa,
                ..
            },
            HirTy::Struct {
                entity: eb,
                args: ab,
                ..
            },
        ) => ea == eb && args_compatible(aa, ab),
        (
            HirTy::Enum {
                entity: ea,
                args: aa,
                ..
            },
            HirTy::Enum {
                entity: eb,
                args: ab,
                ..
            },
        ) => ea == eb && args_compatible(aa, ab),
        (
            HirTy::Protocol {
                entity: ea,
                args: aa,
                ..
            },
            HirTy::Protocol {
                entity: eb,
                args: ab,
                ..
            },
        ) => ea == eb && args_compatible(aa, ab),
        (
            HirTy::AliasUse {
                entity: ea,
                args: aa,
                ..
            },
            HirTy::AliasUse {
                entity: eb,
                args: ab,
                ..
            },
        ) => ea == eb && args_compatible(aa, ab),
        (HirTy::Tuple(aa, _), HirTy::Tuple(ab, _)) => args_compatible(aa, ab),
        (
            HirTy::Function {
                params: pa,
                ret: ra,
                ..
            },
            HirTy::Function {
                params: pb,
                ret: rb,
                ..
            },
        ) => args_compatible(pa, pb) && ty_shape_compatible(ra, rb),
        (HirTy::Never(_), HirTy::Never(_)) => true,
        _ => false,
    }
}

fn args_compatible(a: &[HirTy], b: &[HirTy]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| ty_shape_compatible(x, y))
}

fn is_shape_wildcard(t: &HirTy) -> bool {
    matches!(
        t,
        HirTy::Param(..)
            | HirTy::SelfType(..)
            | HirTy::AssocProjection { .. }
            | HirTy::Infer(..)
            | HirTy::Error(..)
    )
}

/// Extract type arguments from a protocol type in a where clause.
/// E.g., for `Factory[lang.i64]`, returns `[HirTy for lang.i64]`.
fn extract_protocol_type_args(
    ctx: &QueryContext<'_>,
    owner: Entity,
    root: Entity,
    protocol_ty: &AstType,
) -> Vec<HirTy> {
    match protocol_ty {
        AstType::Named { segments, .. } => {
            let result: Vec<HirTy> = segments
                .last()
                .map(|seg| {
                    seg.type_args
                        .iter()
                        .map(|a| kestrel_hir_lower::lower_ast_type(ctx, owner, root, a))
                        .collect()
                })
                .unwrap_or_default();
            result
        },
        _ => Vec::new(),
    }
}

impl WorldResolver<'_> {
    /// Probe for a member that `VisibleChildrenByName` filtered out due to
    /// visibility. Walks direct children of the type and its extensions for a
    /// name match, returning the first hit along with its declared `Vis`.
    /// Used only when normal resolution came up empty, so the diagnostic can
    /// distinguish "is private" from "no member".
    fn find_hidden_member(
        &self,
        entity: Entity,
        name: &str,
        extensions: &[Entity],
    ) -> Option<(Entity, Vis)> {
        let probe = |parent: Entity| -> Option<(Entity, Vis)> {
            for &child in self.ctx.children_of(parent) {
                if self.ctx.get::<Name>(child).is_some_and(|n| n.0 == name) {
                    let vis = self.ctx.get::<Vis>(child).cloned().unwrap_or(Vis::Public);
                    return Some((child, vis));
                }
            }
            None
        };
        if let Some(hit) = probe(entity) {
            return Some(hit);
        }
        for &ext in extensions {
            if let Some(hit) = probe(ext) {
                return Some(hit);
            }
        }
        None
    }

    /// Search children of an entity for a TypeAlias with the given name.
    fn find_associated_type_in_entity(
        &self,
        entity: Entity,
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        for &child in self.ctx.children_of(entity) {
            if self.ctx.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && self.ctx.get::<Name>(child).is_some_and(|n| n.0 == name)
            {
                // Try to lower the TypeAnnotation (concrete associated type)
                if let Some(hir_ty) = self.ctx.query(LowerTypeAnnotation {
                    entity: child,
                    root: self.root,
                }) {
                    return Some(AssociatedTypeResolution {
                        resolved: hir_ty,
                        source_extension: None,
                    });
                }
                // Abstract associated type (no TypeAnnotation) — keep as AliasUse
                // so the solver can detect the missing definition and dispatch via
                // protocol bounds. (Inference reduces AliasUse with a TypeAnnotation;
                // an AliasUse without one stays abstract.)
                return Some(AssociatedTypeResolution {
                    resolved: kestrel_hir::ty::HirTy::AliasUse {
                        entity: child,
                        args: vec![],
                        span: kestrel_span::Span::synthetic(0),
                    },
                    source_extension: None,
                });
            }
        }
        None
    }

    /// Search a protocol for an associated type, including inherited protocols
    /// and protocol extensions.
    ///
    /// TODO: Move this into a shared name-resolution query that returns a
    /// single associated-type member by name, so type inference does not need
    /// to know how protocol associated-type traversal is assembled.
    fn find_associated_type_in_protocol(
        &self,
        protocol: Entity,
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        self.find_associated_type_in_protocols(&[protocol], name)
    }

    /// Search protocol bounds for an associated type with the given name.
    /// Uses `ProtocolAssociatedTypes` which walks protocol direct children,
    /// extension defaults, parent protocols, and their extensions in one pass.
    /// Qualified associated types (`type Equal.Output = Bool`) are excluded —
    /// they bind a *specific* protocol's assoc type and must not leak into
    /// unqualified `T.Output` lookups.
    fn find_associated_type_in_protocols(
        &self,
        protocols: &[Entity],
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        for &proto in protocols {
            let members = self.ctx.query(kestrel_name_res::ProtocolAssociatedTypes {
                protocol: proto,
                root: self.root,
            });
            for m in members {
                if !self.ctx.get::<Name>(m.entity).is_some_and(|n| n.0 == name) {
                    continue;
                }
                // Concrete (has TypeAnnotation) → lower and return.
                if let Some(hir_ty) = self.ctx.query(LowerTypeAnnotation {
                    entity: m.entity,
                    root: self.root,
                }) {
                    return Some(AssociatedTypeResolution {
                        resolved: hir_ty,
                        source_extension: None,
                    });
                }
                // Abstract associated type — keep as AliasUse so the solver
                // detects the missing definition and dispatches via bounds.
                return Some(AssociatedTypeResolution {
                    resolved: kestrel_hir::ty::HirTy::AliasUse {
                        entity: m.entity,
                        args: vec![],
                        span: kestrel_span::Span::synthetic(0),
                    },
                    source_extension: None,
                });
            }
        }
        None
    }

    /// Build a MemberResolution from a resolved member entity.
    fn build_member_resolution(&self, member: Entity) -> Result<MemberResolution, MemberError> {
        let kind = self.ctx.get::<NodeKind>(member);

        // Determine MemberKind and extract type info
        let member_kind = match kind {
            Some(NodeKind::Field) => {
                let mutable = self.ctx.has::<Settable>(member);
                MemberKind::Field { mutable }
            },
            Some(NodeKind::Function) => MemberKind::Method,
            Some(NodeKind::Initializer) => MemberKind::Init,
            Some(NodeKind::Subscript) => MemberKind::Subscript,
            _ if self.ctx.has::<Gettable>(member) => MemberKind::ComputedProperty {
                has_setter: self.ctx.has::<Settable>(member),
            },
            _ => MemberKind::Method, // default to method
        };

        // Get type parameters. When the member sits inside an extension
        // that introduces its own free type params from the conformance RHS
        // (e.g., `extend Int64: ArrayIndex[T]`), append those so the
        // call-site gets fresh inferable TyVars for them — they can't be
        // substituted from the receiver (concrete target like `Int64` carries
        // no args).
        let mut type_params: Vec<Entity> = self
            .ctx
            .get::<kestrel_ast_builder::TypeParams>(member)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();
        if let Some(parent) = self.ctx.parent_of(member) {
            if matches!(self.ctx.get::<NodeKind>(parent), Some(NodeKind::Extension)) {
                if let Some(ext_params) = self.ctx.get::<kestrel_ast_builder::TypeParams>(parent) {
                    for &tp in &ext_params.0 {
                        if !type_params.contains(&tp) {
                            type_params.push(tp);
                        }
                    }
                }
            }
        }

        // Build parameter types from Callable component + lowered types
        let lowered_param_tys = self.ctx.query(LowerCallableTypes {
            entity: member,
            root: self.root,
        });
        let param_types: Vec<ParamInfo> = if let Some(callable) = self.ctx.get::<Callable>(member) {
            let hir_tys = lowered_param_tys.as_ref();
            callable
                .params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let ty = hir_tys
                        .and_then(|tys| tys.get(i))
                        .and_then(|t| t.as_ref())
                        .cloned()
                        .unwrap_or(HirTy::Error(Span::synthetic(0)));
                    ParamInfo {
                        label: p.label.clone(),
                        ty,
                        has_default: p.default_entity.is_some(),
                    }
                })
                .collect()
        } else {
            vec![]
        };

        // Build return type. Fields/properties are typed by their annotation
        // (no annotation = Error; a field without a type is malformed).
        // Callables go through the central `LowerCallableReturnType` query
        // so the unit default is applied in one place.
        let return_type = match &member_kind {
            MemberKind::Field { .. } | MemberKind::ComputedProperty { .. } => self
                .ctx
                .query(LowerTypeAnnotation {
                    entity: member,
                    root: self.root,
                })
                .unwrap_or(HirTy::Error(Span::synthetic(0))),
            _ => self.ctx.query(LowerCallableReturnType {
                entity: member,
                root: self.root,
            }),
        };

        // Get where clauses
        let where_clauses = self.ctx.query(crate::where_clauses::WhereClausesOf {
            entity: member,
            root: self.root,
        });

        // Determine self_type: what entity `Self` resolves to in this member's scope.
        // For protocol/extension methods, this is the protocol entity (needs substitution).
        // For struct/enum methods, self_type matches the receiver so no substitution needed.
        let self_type = self.find_member_self_type(member);

        Ok(MemberResolution {
            entity: member,
            type_params,
            param_types,
            return_type,
            where_clauses,
            kind: member_kind,
            self_type,
            via_protocol: None,
            protocol_type_args: vec![],
            from_extension: None,
        })
    }

    /// Find what entity `Self` resolves to for a member's enclosing type.
    /// Returns Some(entity) for protocol/extension methods where Self-substitution
    /// is needed (Self = protocol entity, not the concrete receiver).
    fn find_member_self_type(&self, member: Entity) -> Option<Entity> {
        let parent = self.ctx.parent_of(member)?;
        match self.ctx.get::<NodeKind>(parent)? {
            NodeKind::Protocol => Some(parent),
            NodeKind::Extension => {
                // Extension's Self is its target type (the protocol or type)
                self.ctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: parent,
                    root: self.root,
                })
            },
            _ => None, // Struct/Enum — Self matches receiver, no substitution needed
        }
    }

    /// Try to resolve an ambiguous member through protocol conformances.
    ///
    /// When multiple candidates match the same name+labels, check if a single
    /// protocol declares a matching method. If found — and every concrete
    /// match in `concrete_matches` is an implementation of that same protocol
    /// requirement (same parameter bind-names, not just labels) — return the
    /// protocol's abstract method signature with protocol type params as
    /// generics. The solver creates fresh TyVars and infers them from args.
    ///
    /// If any concrete match has a different bind-name signature than the
    /// protocol method, the matches are unrelated overloads that merely share
    /// a label signature; this is a real ambiguity, so we return None and let
    /// the caller propagate MemberError::Ambiguous.
    fn try_resolve_through_protocol(
        &self,
        type_entity: Entity,
        name: &str,
        args: &[crate::constraint::CallArg],
        concrete_matches: &[Entity],
    ) -> Option<MemberResolution> {
        let protocols = self.ctx.query(ConformingProtocols {
            entity: type_entity,
            root: self.root,
        });

        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let mut found: Option<(Entity, Entity)> = None; // (protocol, method)

        for &proto in &protocols {
            // Search protocol's direct children for a matching member
            let candidates = if name == "init" {
                // Inits have no Name — search by NodeKind
                self.ctx
                    .children_of(proto)
                    .iter()
                    .filter(|&&c| self.ctx.get::<NodeKind>(c) == Some(&NodeKind::Initializer))
                    .copied()
                    .collect::<Vec<_>>()
            } else if name == "subscript" {
                self.ctx
                    .children_of(proto)
                    .iter()
                    .filter(|&&c| self.ctx.get::<NodeKind>(c) == Some(&NodeKind::Subscript))
                    .copied()
                    .collect::<Vec<_>>()
            } else {
                self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                    parent: proto,
                    name: name.to_string(),
                    context: self.body_owner,
                })
            };

            // Filter by label match
            let matched: Vec<Entity> = candidates
                .into_iter()
                .filter(|&c| self.matches_labels(c, &arg_labels))
                .collect();

            if matched.len() == 1 {
                if found.is_some() {
                    // Multiple protocols match — can't disambiguate
                    return None;
                }
                found = Some((proto, matched[0]));
            }
        }

        let (protocol, method) = found?;

        // Verify every concrete match is structurally compatible with the
        // protocol method — same parameter type constructors at each position,
        // with protocol type params acting as wildcards. If any concrete
        // match's param types differ in SHAPE from the protocol method's,
        // the matches are unrelated overloads that merely share a label
        // signature (e.g. Array's internal `_arrayLiteralPointer:lang.ptr[T],
        // _arrayLiteralCount:lang.i64` vs its `count:Int64, generator:(Int64)
        // -> T` overload), and this is a real ambiguity — not a case where
        // the abstract protocol method can represent them all.
        let proto_param_tys = self.param_type_shapes(method);
        for &c in concrete_matches {
            if c == method {
                continue;
            }
            let concrete_param_tys = self.param_type_shapes(c);
            if !param_shapes_compatible(&proto_param_tys, &concrete_param_tys) {
                return None;
            }
        }

        // Build MemberResolution from the protocol's abstract method
        // Use the protocol's type params (not the method's) for generic resolution
        let proto_type_params: Vec<Entity> = self
            .ctx
            .get::<TypeParams>(protocol)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();

        // Also include the method's own type params (if any)
        let method_type_params: Vec<Entity> = self
            .ctx
            .get::<TypeParams>(method)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();

        let mut all_type_params = proto_type_params;
        all_type_params.extend(method_type_params);

        // Lower param types and return type from the protocol method
        let param_types = self.build_param_types(method);
        let return_type = self.ctx.query(LowerCallableReturnType {
            entity: method,
            root: self.root,
        });

        let where_clauses = self.ctx.query(crate::where_clauses::WhereClausesOf {
            entity: method,
            root: self.root,
        });

        // Determine member kind
        let kind = match self.ctx.get::<NodeKind>(method) {
            Some(NodeKind::Initializer) => MemberKind::Init,
            Some(NodeKind::Subscript) => MemberKind::Subscript,
            _ => MemberKind::Method,
        };

        Some(MemberResolution {
            entity: method,
            type_params: all_type_params,
            param_types,
            return_type,
            where_clauses,
            kind,
            self_type: Some(protocol),
            via_protocol: Some(protocol),
            protocol_type_args: vec![],
            from_extension: None,
        })
    }

    /// Build parameter info list from a callable entity.
    fn build_param_types(&self, entity: Entity) -> Vec<ParamInfo> {
        let Some(callable) = self.ctx.get::<Callable>(entity) else {
            return Vec::new();
        };
        let lowered = self.ctx.query(LowerCallableTypes {
            entity,
            root: self.root,
        });
        callable
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| ParamInfo {
                label: p.label.clone(),
                ty: lowered
                    .as_ref()
                    .and_then(|tys| tys.get(i))
                    .and_then(|t| t.clone())
                    .unwrap_or(HirTy::Error(Span::synthetic(0))),
                has_default: p.default_entity.is_some(),
            })
            .collect()
    }

    /// Check if a callable's parameter labels match the given argument labels.
    /// Handles arity with default parameters.
    fn matches_labels(&self, entity: Entity, arg_labels: &[Option<&str>]) -> bool {
        // Fields and computed properties carry `Callable { params: [] }` for
        // the getter's signature, but for member-resolution purposes they are
        // value accesses, not function calls. Trailing `(args)` after a
        // property is the property's value being called/subscripted — which
        // the solver handles in the field-as-call path. Match any args here
        // and let downstream dispatch decide.
        if self.ctx.get::<NodeKind>(entity) == Some(&NodeKind::Field) {
            return true;
        }
        let Some(callable) = self.ctx.get::<Callable>(entity) else {
            // Non-callable members (fields, properties) match if no args
            return arg_labels.is_empty();
        };

        let params = &callable.params;
        let required_count = params.iter().filter(|p| p.default_entity.is_none()).count();

        // Arity check: args must be >= required and <= total params
        if arg_labels.len() < required_count || arg_labels.len() > params.len() {
            return false;
        }

        // Label check: each arg label must match the corresponding param label
        for (i, arg_label) in arg_labels.iter().enumerate() {
            if i >= params.len() {
                return false;
            }
            let param_label = params[i].label.as_deref();
            if *arg_label != param_label {
                return false;
            }
        }

        true
    }

    /// Check whether an entity could accept `arg_count` args based on arity alone
    /// (ignores labels). Used to gate the single-candidate fallback in member
    /// resolution: if arity doesn't even match, don't accept the candidate.
    ///
    /// For non-callable members (fields, properties), always returns true — the
    /// solver handles the "field used as call" pattern (e.g., `self.data(idx)`
    /// where `data: Array[T]` and `(idx)` is an Array subscript) downstream.
    fn matches_arity(&self, entity: Entity, arg_count: usize) -> bool {
        // See matches_labels: fields/properties are value accesses regardless
        // of the synthesized 0-arity Callable on computed properties.
        if self.ctx.get::<NodeKind>(entity) == Some(&NodeKind::Field) {
            return true;
        }
        let Some(callable) = self.ctx.get::<Callable>(entity) else {
            return true;
        };
        let params = &callable.params;
        let required = params.iter().filter(|p| p.default_entity.is_none()).count();
        arg_count >= required && arg_count <= params.len()
    }

    /// Build a signature string from a callable's param labels for deduplication.
    /// Two methods with the same name and label signature are considered equivalent overloads.
    /// Includes method name to avoid collisions between different no-arg methods.
    fn label_signature(&self, entity: Entity) -> String {
        let name = self
            .ctx
            .get::<Name>(entity)
            .map(|n| n.0.as_str())
            .unwrap_or("");
        let Some(callable) = self.ctx.get::<Callable>(entity) else {
            return name.to_string();
        };
        let labels = callable
            .params
            .iter()
            .map(|p| p.label.as_deref().unwrap_or("_"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}({})", name, labels)
    }

    /// Lowered parameter types (one per param position). Used to check
    /// whether a concrete overload's shape matches a protocol requirement's
    /// shape — see `param_shapes_compatible`.
    fn param_type_shapes(&self, entity: Entity) -> Vec<HirTy> {
        let lowered = self
            .ctx
            .query(LowerCallableTypes {
                entity,
                root: self.root,
            })
            .unwrap_or_default();
        lowered
            .into_iter()
            .map(|t| t.unwrap_or(HirTy::Error(Span::synthetic(0))))
            .collect()
    }

    /// Direct-only: does this extension entity declare a conformance to
    /// `protocol` in its own `Conformances` list?
    ///
    /// Intentionally does NOT walk inheritance or other extensions — callers
    /// that want the full transitive set must use `ConformingProtocols`.
    /// Used by `check_promotion` to identify which extension directly declares
    /// `FromValue` so we can search it for the corresponding static `from`
    /// method.
    fn extension_directly_conforms_to(&self, entity: Entity, protocol: Entity) -> bool {
        let Some(conformances) = self.ctx.get::<Conformances>(entity) else {
            return false;
        };

        for item in &conformances.0 {
            if let ConformanceItem::Positive(ast_ty, _) = item {
                if let Some(resolved) = self.resolve_type_entity(ast_ty) {
                    if resolved == protocol {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Resolve an AstType to a type entity using ResolveTypePath. Resolution
    /// starts from `self.body_owner` (the body being inferred). For type-definition-
    /// scoped resolution (e.g. where clauses), use the `WhereClausesOf` query
    /// instead of threading `self.body_owner` through to non-body contexts.
    fn resolve_type_entity(&self, ast_ty: &kestrel_ast_builder::AstType) -> Option<Entity> {
        use kestrel_ast_builder::AstType;
        match ast_ty {
            AstType::Named { segments, .. } => {
                let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
                match self.ctx.query(ResolveTypePath {
                    segments: seg_names,
                    context: self.body_owner,
                    root: self.root,
                }) {
                    TypeResolution::Found(entity) => Some(entity),
                    TypeResolution::SelfType => self.resolve_self_entity_from(self.body_owner),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    /// Resolve `Self` to the enclosing type entity, starting from `start`.
    fn resolve_self_entity_from(&self, start: Entity) -> Option<Entity> {
        let mut current = Some(start);
        while let Some(entity) = current {
            match self.ctx.get::<NodeKind>(entity) {
                Some(NodeKind::Extension) => {
                    return self.ctx.query(kestrel_name_res::ExtensionTargetEntity {
                        extension: entity,
                        root: self.root,
                    });
                },
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
                    return Some(entity);
                },
                _ => current = self.ctx.parent_of(entity),
            }
        }
        None
    }

    /// Resolve a member on a type parameter by searching its protocol bounds.
    /// E.g., for `T: Iterable`, calling `.iter()` on T searches the Iterable
    /// protocol and its extensions.
    fn resolve_param_member(
        &self,
        param_entity: Entity,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        let direct_bounds = self.collect_param_direct_bounds(param_entity);
        if direct_bounds.is_empty() && self.collect_param_protocol_bounds(param_entity).is_empty() {
            return Err(MemberError::NotFound);
        }

        // Prefer candidates from protocols the param is *directly* bounded by
        // (e.g., `T: Equatable`) over candidates only reached through parent-
        // protocol inheritance or `extend P: Q` conformance extensions. This
        // resolves name collisions between a primary bound and its inherited
        // conformances (e.g., Equatable.equals vs Equal.equals introduced by
        // `extend Equatable: Equal[Self]`).
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let member =
            if let Some(m) = self.select_bound_candidate(&direct_bounds, name, &arg_labels)? {
                m
            } else {
                let expanded = self.collect_param_protocol_bounds(param_entity);
                match self.select_bound_candidate(&expanded, name, &arg_labels)? {
                    Some(m) => m,
                    None => return Err(MemberError::NotFound),
                }
            };
        let mut resolution = self.build_member_resolution(member)?;

        // Attach protocol type args from the where clause bound.
        // E.g., for `F: Factory[lang.i64]`, the method's protocol is Factory,
        // and its type args are [lang.i64]. These substitute the protocol's
        // type params (T → i64) in the method's return/param types.
        if let Some(self_entity) = resolution.self_type {
            resolution.protocol_type_args =
                self.find_protocol_type_args_from_bounds(param_entity, self_entity);
        }

        Ok(resolution)
    }

    /// Find the type arguments for a protocol bound on a type parameter.
    /// Searches the owner's where clauses for `param: Protocol[Args]`,
    /// and also checks inherited protocol conformances (e.g., IntConverter: Converter[i64]).
    fn find_protocol_type_args_from_bounds(
        &self,
        param_entity: Entity,
        protocol_entity: Entity,
    ) -> Vec<HirTy> {
        // Walk up from the param to find the owner (function/init that declares the where clause)
        let owner = self.ctx.parent_of(param_entity).unwrap_or(self.body_owner);
        let clauses = self.ctx.query(crate::where_clauses::WhereClausesOf {
            entity: owner,
            root: self.root,
        });

        // Direct match: where clause says T: Protocol[Args]
        for clause in &clauses {
            if let WhereClause::Bound {
                param,
                protocol,
                protocol_type_args,
                ..
            } = clause
            {
                if *param == param_entity && *protocol == protocol_entity {
                    return protocol_type_args.clone();
                }
            }
        }

        // Inherited match: where clause says T: ParentProtocol, and
        // ParentProtocol: Protocol[Args] in its conformance list.
        // E.g., T: IntConverter, IntConverter: Converter[i64] → find [i64] for Converter.
        for clause in &clauses {
            if let WhereClause::Bound {
                param, protocol, ..
            } = clause
            {
                if *param == param_entity {
                    if let Some(args) =
                        self.find_inherited_protocol_type_args(*protocol, protocol_entity)
                    {
                        return args;
                    }
                }
            }
        }

        Vec::new()
    }

    /// Search a protocol's conformance chain for inherited type args.
    /// E.g., IntConverter: Converter[i64] → returns [i64] when searching for Converter.
    fn find_inherited_protocol_type_args(
        &self,
        from_protocol: Entity,
        target_protocol: Entity,
    ) -> Option<Vec<HirTy>> {
        let conformances = self.ctx.get::<Conformances>(from_protocol)?;
        for item in &conformances.0 {
            let ConformanceItem::Positive(ast_ty, _) = item else {
                continue;
            };
            let Some(resolved) = self.resolve_type_entity(ast_ty) else {
                continue;
            };
            if resolved == target_protocol {
                // Extract type args from the conformance path
                return Some(extract_protocol_type_args(
                    self.ctx,
                    self.body_owner,
                    self.root,
                    ast_ty,
                ));
            }
            // Recurse into inherited protocols
            if self.ctx.get::<NodeKind>(resolved) == Some(&NodeKind::Protocol) {
                if let Some(args) =
                    self.find_inherited_protocol_type_args(resolved, target_protocol)
                {
                    return Some(args);
                }
            }
        }
        None
    }

    /// Search a set of protocols (and their extensions) for a member by name.
    /// Handles named members via VisibleChildrenByName and subscripts/inits
    /// by NodeKind when using sentinel names.
    fn search_protocols_for_member(&self, protocols: &[Entity], name: &str) -> Vec<Entity> {
        // Dedup within a protocol by signature (inherited methods collapse).
        // Across protocols, dedup by entity only: the same entity reached
        // through two bounds (e.g., `I: Iterator, I: DoubleEndedIterator` both
        // surfacing `Iterator.next`) is one method, not an ambiguity. Distinct
        // entities with the same signature still surface as ambiguity.
        //
        // `ProtocolMembersByName` handles direct children, extension defaults,
        // parent protocols, and the `"init"` / `"subscript"` sentinels for
        // nameless Callable entities.
        let mut all_candidates = Vec::new();
        let mut seen_entities = std::collections::HashSet::new();
        for proto in protocols {
            let mut seen_in_proto = std::collections::HashSet::new();
            let members = self.ctx.query(kestrel_name_res::ProtocolMembersByName {
                protocol: *proto,
                name: name.to_string(),
                context: self.body_owner,
                root: self.root,
            });
            for m in members {
                let sig = self.label_signature(m.entity);
                if seen_in_proto.insert(sig) && seen_entities.insert(m.entity) {
                    all_candidates.push(m.entity);
                }
            }
        }

        all_candidates
    }

    /// Search a list of protocols for a label-matching member. Used by
    /// param/associated-type member resolution to try direct bounds before
    /// falling back to transitively-expanded bounds.
    ///
    /// Returns:
    ///   - `Ok(Some(entity))` — exactly one candidate matches by labels/arity.
    ///   - `Ok(None)` — no candidates at all (caller may try a broader bound set).
    ///   - `Err(Ambiguous)` — multiple candidates match; genuine ambiguity among
    ///     this bound set, so the caller should propagate rather than widen.
    fn select_bound_candidate(
        &self,
        protocols: &[Entity],
        name: &str,
        arg_labels: &[Option<&str>],
    ) -> Result<Option<Entity>, MemberError> {
        if protocols.is_empty() {
            return Ok(None);
        }
        let all_candidates = self.search_protocols_for_member(protocols, name);
        if all_candidates.is_empty() {
            return Ok(None);
        }
        let matches: Vec<Entity> = all_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, arg_labels))
            .collect();
        match matches.len() {
            0 => {
                // If there's exactly one candidate, accept it even when labels
                // don't match so downstream label-checking can report the real
                // "wrong argument label" diagnostic instead of "no member".
                if all_candidates.len() == 1 {
                    Ok(Some(all_candidates[0]))
                } else {
                    Ok(None)
                }
            },
            1 => Ok(Some(matches[0])),
            _ => Err(MemberError::Ambiguous(
                matches.into_iter().map(|e| (e, None, 0)).collect(),
            )),
        }
    }

    /// From a list of candidates, filter by instance (non-static), match labels,
    /// and select a single member. Returns (instance_candidates, chosen_member).
    fn select_member_candidate(
        &self,
        all_candidates: Vec<Entity>,
        args: &[crate::constraint::CallArg],
    ) -> Result<(Vec<Entity>, Entity), MemberError> {
        let instance_candidates: Vec<Entity> = all_candidates
            .into_iter()
            .filter(|&c| !self.ctx.has::<Static>(c))
            .collect();

        if instance_candidates.is_empty() {
            return Err(MemberError::NotFound);
        }

        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let matches: Vec<Entity> = instance_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, &arg_labels))
            .collect();

        let member = match matches.len() {
            0 => {
                if instance_candidates.len() == 1 {
                    instance_candidates[0]
                } else {
                    return Err(MemberError::NotFound);
                }
            },
            1 => matches[0],
            _ => {
                return Err(MemberError::Ambiguous(
                    matches.into_iter().map(|e| (e, None, 0)).collect(),
                ));
            },
        };

        Ok((instance_candidates, member))
    }

    /// Resolve a member on an associated type (TypeAlias entity, e.g., `Iter`, `Item`).
    ///
    /// Associated types in protocols have bounds (e.g., `type Iter: Iterator`).
    /// We collect protocol bounds from:
    /// - The TypeAlias's Conformances component (if present)
    /// - The parent protocol's where clause (`where Iter: Iterator`)
    /// Then search those protocols for the member, same as resolve_param_member.
    fn resolve_assoc_type_member(
        &self,
        alias_entity: Entity,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        // Prefer direct bounds over expanded bounds (same reason as
        // resolve_param_member — see the comment there).
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let direct = self.collect_assoc_type_direct_bounds(alias_entity);
        if let Some(m) = self.select_bound_candidate(&direct, name, &arg_labels)? {
            return self.build_member_resolution(m);
        }
        let expanded = self.collect_assoc_type_protocol_bounds(alias_entity);
        if expanded.is_empty() {
            return Err(MemberError::NotFound);
        }
        let all_candidates = self.search_protocols_for_member(&expanded, name);
        let (_, member) = self.select_member_candidate(all_candidates, args)?;
        self.build_member_resolution(member)
    }

    /// Resolve a STATIC member on an associated type (e.g., `Item.zero`).
    /// Like resolve_assoc_type_member but searches static members only.
    fn resolve_assoc_type_static_member_resolve(
        &self,
        alias_entity: Entity,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        let bound_protocols = self.collect_assoc_type_protocol_bounds(alias_entity);
        if bound_protocols.is_empty() {
            return Err(MemberError::NotFound);
        }

        let all_candidates = self.search_protocols_for_member(&bound_protocols, name);

        // Filter to static members only
        let static_candidates: Vec<Entity> = all_candidates
            .into_iter()
            .filter(|&c| self.ctx.has::<Static>(c))
            .collect();

        if static_candidates.is_empty() {
            return Err(MemberError::NotFound);
        }

        // Label matching for static members
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let matches: Vec<Entity> = static_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, &arg_labels))
            .collect();

        let member = match matches.len() {
            0 if static_candidates.len() == 1 => static_candidates[0],
            0 => return Err(MemberError::NotFound),
            1 => matches[0],
            _ => {
                return Err(MemberError::Ambiguous(
                    matches.into_iter().map(|e| (e, None, 0)).collect(),
                ));
            },
        };

        self.build_member_resolution(member)
    }

    /// Collect protocol bounds on an associated type (TypeAlias).
    ///
    /// Searches the TypeAlias's Conformances component, the parent protocol's
    /// where clause, and the owner hierarchy for bounds like `Iter: Iterator`
    /// or `where Item: Equatable`.
    fn collect_assoc_type_protocol_bounds(&self, alias_entity: Entity) -> Vec<Entity> {
        let (mut protocols, mut visited) =
            self.collect_assoc_type_direct_bounds_inner(alias_entity);
        expand_protocol_closure_in_place(self.ctx, self.root, &mut protocols, &mut visited);
        protocols
    }

    /// Direct bounds on an associated type — Conformances, parent protocol's
    /// where clause, and owner-hierarchy where clauses — without walking
    /// parent-protocol or conformance-extension closures.
    fn collect_assoc_type_direct_bounds(&self, alias_entity: Entity) -> Vec<Entity> {
        self.collect_assoc_type_direct_bounds_inner(alias_entity).0
    }

    fn collect_assoc_type_direct_bounds_inner(
        &self,
        alias_entity: Entity,
    ) -> (Vec<Entity>, std::collections::HashSet<Entity>) {
        let mut protocols = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut checked = std::collections::HashSet::new();

        // Check Conformances on the TypeAlias itself (e.g., `type Iter: Iterator`)
        if let Some(conformances) = self.ctx.get::<Conformances>(alias_entity) {
            for item in &conformances.0 {
                if let ConformanceItem::Positive(ast_ty, _) = item {
                    if let Some(proto) = self.resolve_type_entity(ast_ty) {
                        if self.ctx.get::<NodeKind>(proto) == Some(&NodeKind::Protocol) {
                            if visited.insert(proto) {
                                protocols.push(proto);
                            }
                        }
                    }
                }
            }
        }

        // Check parent protocol's where clause for bounds on this alias.
        // E.g., `protocol Iterable { type Iter }` with `where Iter: Iterator`
        if let Some(parent) = self.ctx.parent_of(alias_entity) {
            checked.insert(parent);
            // Reuse the same where clause extraction — it resolves subjects by entity
            self.gather_bounds_from_where_clause(
                alias_entity,
                parent,
                &mut protocols,
                &mut visited,
            );
        }

        // Walk from owner upward — function/extension where clauses may also
        // constrain associated types. E.g., `func contains() where Item: Equatable`
        let mut current = Some(self.body_owner);
        while let Some(entity) = current {
            if checked.insert(entity) {
                self.gather_bounds_from_where_clause(
                    alias_entity,
                    entity,
                    &mut protocols,
                    &mut visited,
                );
            }
            current = self.ctx.parent_of(entity);
        }

        (protocols, visited)
    }

    /// Collect all protocol entities a type parameter is bound to,
    /// from where clauses on the param's parent AND ancestor entities of
    /// the current owner (including transitive protocol inheritance).
    ///
    /// Collect additional protocols from enclosing extension where clauses.
    /// For `extend Filterable where Self: Sortable`, returns `[Sortable]` when
    /// called with `target_protocol = Filterable`.
    fn collect_extension_where_clause_protocols(&self, target_protocol: Entity) -> Vec<Entity> {
        let mut protocols = Vec::new();

        // Walk from owner up to find the enclosing extension
        let mut current = Some(self.body_owner);
        while let Some(entity) = current {
            if self.ctx.get::<NodeKind>(entity) == Some(&NodeKind::Extension) {
                // Check if this extension targets our protocol
                let ext_target = self.ctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: entity,
                    root: self.root,
                });
                if ext_target == Some(target_protocol) {
                    // Get where clause bounds where the subject is the target protocol (Self)
                    let clauses = self.ctx.query(crate::where_clauses::WhereClausesOf {
                        entity,
                        root: self.root,
                    });
                    for clause in clauses {
                        if let WhereClause::Bound {
                            param, protocol, ..
                        } = clause
                        {
                            // `Self: Protocol` — param is the target protocol entity
                            if param == target_protocol {
                                protocols.push(protocol);
                            }
                        }
                    }
                }
                break; // Found the extension, stop walking
            }
            current = self.ctx.parent_of(entity);
        }

        protocols
    }

    /// Where clauses can live on the param's direct parent (function/method),
    /// on an extension (`extend Array[T] where T: Comparable`), or on any
    /// ancestor entity of the owner being compiled.
    fn collect_param_protocol_bounds(&self, param_entity: Entity) -> Vec<Entity> {
        let (mut protocols, mut visited) = self.collect_param_direct_bounds_inner(param_entity);

        // Walk inherited protocols + extension-added conformances transitively.
        expand_protocol_closure_in_place(self.ctx, self.root, &mut protocols, &mut visited);

        protocols
    }

    /// Collect only the protocols directly listed as bounds on `param_entity`
    /// in where clauses on the param's parent and the owner hierarchy.
    /// Does NOT walk parent-protocol inheritance or conformance extensions.
    /// Used to prefer direct bounds when a method name is declared on both a
    /// direct bound and an expanded bound (e.g., Equatable.equals vs Equal.equals
    /// reached via `extend Equatable: Equal[Self]`).
    fn collect_param_direct_bounds(&self, param_entity: Entity) -> Vec<Entity> {
        self.collect_param_direct_bounds_inner(param_entity).0
    }

    fn collect_param_direct_bounds_inner(
        &self,
        param_entity: Entity,
    ) -> (Vec<Entity>, std::collections::HashSet<Entity>) {
        let mut protocols = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut checked = std::collections::HashSet::new();

        // Check param's direct parent (function/method that declares the type param)
        if let Some(parent) = self.ctx.parent_of(param_entity) {
            checked.insert(parent);
            self.gather_bounds_from_where_clause(
                param_entity,
                parent,
                &mut protocols,
                &mut visited,
            );
        }

        // Walk from owner upward to find extension/protocol where clauses
        // that also constrain this type param. E.g., the method being compiled
        // is inside `extend Array[T] where T: Comparable`, and T is Array's param.
        let mut current = Some(self.body_owner);
        while let Some(entity) = current {
            if checked.insert(entity) {
                self.gather_bounds_from_where_clause(
                    param_entity,
                    entity,
                    &mut protocols,
                    &mut visited,
                );
            }
            current = self.ctx.parent_of(entity);
        }

        (protocols, visited)
    }

    /// Extract protocol bounds for `param_entity` from a single entity's WhereClause.
    fn gather_bounds_from_where_clause(
        &self,
        param_entity: Entity,
        entity: Entity,
        protocols: &mut Vec<Entity>,
        visited: &mut std::collections::HashSet<Entity>,
    ) {
        let Some(wc) = self.ctx.get::<AstWhereClause>(entity) else {
            return;
        };
        for constraint in &wc.0 {
            if let WhereConstraint::Bound {
                subject,
                protocols: proto_types,
                ..
            } = constraint
            {
                if let Some(resolved_subj) = self.resolve_type_entity(subject) {
                    if resolved_subj == param_entity {
                        for proto_ty in proto_types {
                            if let Some(proto) = self.resolve_type_entity(proto_ty) {
                                if visited.insert(proto) {
                                    protocols.push(proto);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Rank ambiguous candidates by extension specificity.
    /// Returns the winning candidate if one extension is strictly more specific.
    /// Specificity = number of concrete (non-type-parameter) type args in the extension target.
    /// Rank candidates by extension specificity (most specific first).
    /// Returns (candidate, from_extension) pairs sorted descending by concrete type arg count.
    fn rank_by_extension_specificity(
        &self,
        matches: &[Entity],
        candidate_extensions: &[(Entity, Entity)],
    ) -> Vec<(Entity, Option<Entity>, usize)> {
        let mut scored: Vec<(Entity, Option<Entity>, usize)> = Vec::new();

        for &candidate in matches {
            let ext = candidate_extensions
                .iter()
                .find(|(c, _)| *c == candidate)
                .map(|&(_, e)| e);

            let specificity = ext
                .and_then(|e| {
                    self.ctx.query(LowerExtensionTargetTypeArgs {
                        extension: e,
                        root: self.root,
                    })
                })
                .map(|args| {
                    args.iter()
                        .filter(|t| !matches!(t, HirTy::Param(..)))
                        .count()
                })
                .unwrap_or(0);

            scored.push((candidate, ext, specificity));
        }

        scored.sort_by_key(|(_, _, s)| std::cmp::Reverse(*s));
        scored
    }
}
