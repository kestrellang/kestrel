//! Type resolver trait and world implementation.
//!
//! `TypeResolver` abstracts world queries for testability. The solver
//! depends on this trait, not on concrete `QueryContext`. `WorldResolver`
//! is the real implementation; tests can provide mocks.

use kestrel_ast_builder::{
    AstType, Callable, Conformances, ConformanceItem, Gettable, Name, NodeKind, Settable, Static,
    TypeParams, WhereClause as AstWhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin, ResolveTypePath, TypeResolution};
use kestrel_span2::Span;

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
    Ambiguous(Vec<Entity>),
    NotVisible,
}

/// Result of resolving an associated type.
#[derive(Clone, Debug)]
pub struct AssociatedTypeResolution {
    /// The concrete type this associated type resolves to.
    pub resolved: HirTy,
}

/// Where clause on a declaration.
#[derive(Clone, Debug)]
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

    /// Get the where clauses for an entity (function, method, init).
    fn where_clauses(&self, entity: Entity) -> Vec<WhereClause>;

    /// Get where clauses resolving names in a specific context entity's scope.
    fn where_clauses_in_context(&self, entity: Entity, context: Entity) -> Vec<WhereClause>;

    /// Check if `to` type can be constructed from `from` type via FromValue promotion.
    /// Returns the `from()` method entity if promotion is possible.
    fn check_promotion(&self, from: &TyKind, to: &TyKind) -> Option<Entity>;
}

// ===== WorldResolver: real implementation over QueryContext =====

/// Implements TypeResolver using the ECS world.
pub struct WorldResolver<'a> {
    pub ctx: &'a QueryContext<'a>,
    pub root: Entity,
    pub owner: Entity,
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

        let TyKind::Named { entity, .. } = receiver_ty else {
            return Err(MemberError::NotFound);
        };

        // TypeParameter entities can arrive as Named (from lower_hir_ty_sub fallthrough).
        // Route to protocol-bound search instead of direct member search.
        if self.ctx.get::<NodeKind>(*entity) == Some(&NodeKind::TypeParameter) {
            return self.resolve_param_member(*entity, name, _args);
        }

        // TypeAlias entities are protocol associated types (e.g., Iter, Item).
        // Search the protocol's bounds on that associated type for the member.
        // If instance member search fails, retry with static members (e.g., Item.zero).
        if self.ctx.get::<NodeKind>(*entity) == Some(&NodeKind::TypeAlias) {
            match self.resolve_assoc_type_member(*entity, name, _args) {
                Ok(res) => return Ok(res),
                Err(MemberError::NotFound) => {
                    // Fall back to static member search (e.g., Item.zero where Item: Addable)
                    return self.resolve_assoc_type_static_member_resolve(*entity, name, _args);
                }
                Err(e) => return Err(e),
            }
        }

        // Search direct children by name
        let candidates = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *entity,
            name: name.to_string(),
            context: self.owner,
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
                context: self.owner,
            });
            for &child in &ext_children {
                candidate_extensions.push((child, *ext));
            }
            all_candidates.extend(ext_children);
        }

        // Fallback: search protocol extensions for default method implementations.
        // E.g., `lessThan` lives in `extend Comparable { ... }`, not in the protocol
        // itself. We only search extensions (not abstract protocol declarations)
        // to avoid ambiguity between the abstract requirement and the default impl.
        //
        // Multiple protocol extensions may provide the same method (e.g. notEquals
        // from both extend Equatable and extend Comparable). These are equivalent
        // default implementations, so we deduplicate by label signature.
        if all_candidates.is_empty() {
            let protocols = self.ctx.query(kestrel_name_res::ConformingProtocols {
                entity: *entity,
                root: self.root,
            });
            let mut proto_candidates = Vec::new();
            for proto in &protocols {
                let proto_extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                    target: *proto,
                    root: self.root,
                });
                for ext in &proto_extensions {
                    let ext_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                        parent: *ext,
                        name: name.to_string(),
                        context: self.owner,
                    });
                    proto_candidates.extend(ext_children);
                }
            }

            // Deduplicate protocol extension candidates by label signature.
            // Same method from different protocol extensions = equivalent default impl.
            let mut seen_signatures = std::collections::HashSet::new();
            for &cand in &proto_candidates {
                let sig = self.label_signature(cand);
                if seen_signatures.insert(sig) {
                    all_candidates.push(cand);
                }
            }
        }

        // If receiver is a protocol and we're inside a protocol extension with
        // where clauses like `Self: Sortable`, also search those constraint protocols.
        if all_candidates.is_empty()
            && self.ctx.get::<NodeKind>(*entity) == Some(&NodeKind::Protocol)
        {
            let extra_protocols = self.collect_extension_where_clause_protocols(*entity);
            for proto in &extra_protocols {
                // Search the protocol's own children
                let proto_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                    parent: *proto,
                    name: name.to_string(),
                    context: self.owner,
                });
                all_candidates.extend(proto_children);

                // Also search extensions of the constraint protocol
                if all_candidates.is_empty() {
                    let proto_extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                        target: *proto,
                        root: self.root,
                    });
                    for ext in &proto_extensions {
                        let ext_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                            parent: *ext,
                            name: name.to_string(),
                            context: self.owner,
                        });
                        all_candidates.extend(ext_children);
                    }
                }
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
        if all_candidates.is_empty() && name == "(subscript)" {
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
                // No label match — fall back to single candidate if only one exists
                if instance_candidates.len() == 1 {
                    instance_candidates[0]
                } else {
                    return Err(MemberError::NotFound);
                }
            }
            1 => matches[0],
            _ => {
                // Multiple candidates with same labels — try protocol-based resolution.
                // If the ambiguous members implement a single protocol requirement,
                // return the protocol's abstract method and let the solver
                // disambiguate via type inference.
                if let Some(proto_res) = self.try_resolve_through_protocol(*entity, name, _args) {
                    return Ok(proto_res);
                }
                return Err(MemberError::Ambiguous(matches));
            }
        };

        let mut resolution = self.build_member_resolution(member)?;
        // Track which extension the member came from (for solver-side type arg filtering)
        if let Some(&(_, ext)) = candidate_extensions.iter().find(|(c, _)| *c == member) {
            resolution.from_extension = Some(ext);
        }
        Ok(resolution)
    }

    fn conforms_to(&self, ty: &TyKind, protocol: Entity) -> bool {
        match ty {
            TyKind::Named { entity, .. } => {
                // TypeParameter entities can arrive as Named (from lower_hir_ty_sub).
                // Check their where clause bounds instead of conformance declarations.
                if self.ctx.get::<NodeKind>(*entity) == Some(&NodeKind::TypeParameter) {
                    let bound_protocols = self.collect_param_protocol_bounds(*entity);
                    return bound_protocols.contains(&protocol);
                }
                // TypeAlias entities (associated types like Item, Iter) — check
                // protocol bounds from conformances and where clauses.
                if self.ctx.get::<NodeKind>(*entity) == Some(&NodeKind::TypeAlias) {
                    let bound_protocols = self.collect_assoc_type_protocol_bounds(*entity);
                    return bound_protocols.contains(&protocol);
                }
                // Walk the full transitive conformance chain (memoized query)
                let all_protocols = self.ctx.query(kestrel_name_res::ConformingProtocols {
                    entity: *entity,
                    root: self.root,
                });
                all_protocols.contains(&protocol)
            }
            TyKind::Param { entity } => {
                // Type parameters: check where clause bounds for protocol conformance
                let bound_protocols = self.collect_param_protocol_bounds(*entity);
                bound_protocols.contains(&protocol)
            }
            _ => false,
        }
    }

    fn resolve_associated_type(
        &self,
        container: &TyKind,
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        match container {
            TyKind::Named { entity, .. } => {
                let node_kind = self.ctx.get::<NodeKind>(*entity);

                // TypeAlias (protocol associated type like Iter) or TypeParameter —
                // search protocol bounds for the associated type.
                // E.g., Iter: Iterator → look for Iterator.Item
                if matches!(node_kind, Some(NodeKind::TypeAlias) | Some(NodeKind::TypeParameter)) {
                    let bound_protocols = if node_kind == Some(&NodeKind::TypeAlias) {
                        self.collect_assoc_type_protocol_bounds(*entity)
                    } else {
                        self.collect_param_protocol_bounds(*entity)
                    };
                    return self.find_associated_type_in_protocols(&bound_protocols, name);
                }

                // Concrete type — search children for a TypeAlias with matching name
                self.find_associated_type_in_entity(*entity, name)
            }
            TyKind::Param { entity } => {
                // Type parameter — search protocol bounds for the associated type
                let bound_protocols = self.collect_param_protocol_bounds(*entity);
                self.find_associated_type_in_protocols(&bound_protocols, name)
            }
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
        let TyKind::Named { entity: to_entity, .. } = to else {
            return None;
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
            if !self.entity_conforms_to(*ext, from_value_protocol) {
                continue;
            }

            // Look for a static `from` method in this extension
            let children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                parent: *ext,
                name: "from".to_string(),
                context: self.owner,
            });

            for &child in &children {
                if self.ctx.has::<Static>(child)
                    && matches!(
                        self.ctx.get::<NodeKind>(child),
                        Some(NodeKind::Function)
                    )
                {
                    return Some(child);
                }
            }
        }

        // Also check direct children (in case FromValue is implemented directly)
        let direct = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *to_entity,
            name: "from".to_string(),
            context: self.owner,
        });

        for &child in &direct {
            if self.ctx.has::<Static>(child)
                && matches!(
                    self.ctx.get::<NodeKind>(child),
                    Some(NodeKind::Function)
                )
            {
                return Some(child);
            }
        }

        None
    }

    fn where_clauses(&self, entity: Entity) -> Vec<WhereClause> {
        self.where_clauses_in_context(entity, self.owner)
    }

    /// Get where clauses, resolving names in a specific context entity's scope.
    fn where_clauses_in_context(&self, entity: Entity, context: Entity) -> Vec<WhereClause> {
        let Some(ast_wc) = self.ctx.get::<AstWhereClause>(entity) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        for constraint in &ast_wc.0 {
            match constraint {
                WhereConstraint::Bound {
                    subject, protocols, ..
                } => {
                    // Resolve subject to a type param entity
                    let Some(param) = self.resolve_type_entity_in_context(subject, context) else {
                        continue;
                    };

                    // Resolve each protocol, including type arguments
                    for protocol_ty in protocols {
                        if let Some(protocol) = self.resolve_type_entity_in_context(protocol_ty, context) {
                            // Extract type args from the protocol type (e.g., Factory[lang.i64])
                            let protocol_type_args = extract_protocol_type_args(
                                self.ctx, context, self.root, protocol_ty,
                            );
                            result.push(WhereClause::Bound { param, protocol, protocol_type_args });
                        }
                    }
                }
                WhereConstraint::Equality { lhs, rhs, .. } => {
                    let rhs_hir = kestrel_hir_lower::lower_ast_type(
                        self.ctx,
                        context,
                        self.root,
                        rhs,
                    );
                    // Resolve the LHS and inspect what it is
                    if let Some((param, assoc_name)) = self.extract_associated_type_path(lhs) {
                        // 2-segment path like T.Item → associated type equality
                        result.push(WhereClause::TypeEquality {
                            param,
                            assoc_name,
                            rhs: rhs_hir,
                        });
                    } else if let Some(param) = self.resolve_type_param_or_assoc(lhs) {
                        // Bare type param (V) or associated type (Item) → direct equality
                        result.push(WhereClause::DirectEquality {
                            param,
                            rhs: rhs_hir,
                        });
                    }
                }
                WhereConstraint::NegativeBound { .. } => {
                    // Negative bounds are not modeled in inference where clauses
                }
            }
        }

        result
    }

    fn resolve_static_member(
        &self,
        receiver_ty: &TyKind,
        name: &str,
        args: &[crate::constraint::CallArg],
    ) -> Result<MemberResolution, MemberError> {
        let TyKind::Named { entity, .. } = receiver_ty else {
            return Err(MemberError::NotFound);
        };

        // Search direct children and extensions for static members
        let mut all_candidates: Vec<kestrel_hecs::Entity> = Vec::new();

        let children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *entity,
            name: name.to_string(),
            context: self.owner,
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
                context: self.owner,
            });
            all_candidates.extend(ext_children.iter());
        }

        // Filter to static members only
        let static_candidates: Vec<kestrel_hecs::Entity> = all_candidates
            .into_iter()
            .filter(|&c| self.ctx.has::<Static>(c))
            .collect();

        if static_candidates.is_empty() {
            return Err(MemberError::NotFound);
        }

        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let matches: Vec<kestrel_hecs::Entity> = static_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, &arg_labels))
            .collect();

        let member = match matches.len() {
            0 => {
                if static_candidates.len() == 1 {
                    static_candidates[0]
                } else {
                    return Err(MemberError::NotFound);
                }
            }
            1 => matches[0],
            _ => return Err(MemberError::Ambiguous(matches)),
        };

        self.build_member_resolution(member)
    }
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
            let result: Vec<HirTy> = segments.last()
                .map(|seg| {
                    seg.type_args.iter()
                        .map(|a| kestrel_hir_lower::lower_ast_type(ctx, owner, root, a))
                        .collect()
                })
                .unwrap_or_default();
            result
        }
        _ => Vec::new(),
    }
}

impl WorldResolver<'_> {
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
                    return Some(AssociatedTypeResolution { resolved: hir_ty });
                }
                // Abstract associated type (no TypeAnnotation) — return as Named entity
                return Some(AssociatedTypeResolution {
                    resolved: kestrel_hir::ty::HirTy::Named {
                        entity: child,
                        args: vec![],
                        span: kestrel_span2::Span::synthetic(0),
                    },
                });
            }
        }
        None
    }

    /// Search protocol bounds for an associated type with the given name.
    fn find_associated_type_in_protocols(
        &self,
        protocols: &[Entity],
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        for &proto in protocols {
            if let Some(result) = self.find_associated_type_in_entity(proto, name) {
                return Some(result);
            }
            // Also check protocol extensions
            let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                target: proto,
                root: self.root,
            });
            for ext in &extensions {
                if let Some(result) = self.find_associated_type_in_entity(*ext, name) {
                    return Some(result);
                }
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
            }
            Some(NodeKind::Function) => MemberKind::Method,
            Some(NodeKind::Initializer) => MemberKind::Init,
            Some(NodeKind::Subscript) => MemberKind::Subscript,
            _ if self.ctx.has::<Gettable>(member) => MemberKind::ComputedProperty {
                has_setter: self.ctx.has::<Settable>(member),
            },
            _ => MemberKind::Method, // default to method
        };

        // Get type parameters
        let type_params: Vec<Entity> = self
            .ctx
            .get::<kestrel_ast_builder::TypeParams>(member)
            .map(|tp| tp.0.clone())
            .unwrap_or_default();

        // Build parameter types from Callable component + lowered types
        let lowered_param_tys = self.ctx.query(LowerCallableTypes {
            entity: member,
            root: self.root,
        });
        let param_types: Vec<ParamInfo> = if let Some(callable) = self.ctx.get::<Callable>(member)
        {
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
                    }
                })
                .collect()
        } else {
            vec![]
        };

        // Build return type from TypeAnnotation
        let return_type = match &member_kind {
            MemberKind::Field { .. } | MemberKind::ComputedProperty { .. } => {
                // Fields/properties: return the field type
                self.ctx
                    .query(LowerTypeAnnotation {
                        entity: member,
                        root: self.root,
                    })
                    .unwrap_or(HirTy::Error(Span::synthetic(0)))
            }
            _ => {
                // Methods/inits/subscripts: return type from annotation
                self.ctx
                    .query(LowerTypeAnnotation {
                        entity: member,
                        root: self.root,
                    })
                    .unwrap_or_else(|| HirTy::Tuple(vec![], Span::synthetic(0)))
            }
        };

        // Get where clauses
        let where_clauses = self.where_clauses(member);

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
            }
            _ => None, // Struct/Enum — Self matches receiver, no substitution needed
        }
    }

    /// Try to resolve an ambiguous member through protocol conformances.
    ///
    /// When multiple candidates match the same name+labels, check if a single
    /// protocol declares a matching method. If found, return the protocol's
    /// abstract method signature with protocol type params as generics.
    /// The solver creates fresh TyVars and infers them from arguments.
    fn try_resolve_through_protocol(
        &self,
        type_entity: Entity,
        name: &str,
        args: &[crate::constraint::CallArg],
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
            } else if name == "(subscript)" {
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
                    context: self.owner,
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
        let return_type = self
            .ctx
            .query(LowerTypeAnnotation {
                entity: method,
                root: self.root,
            })
            .unwrap_or(HirTy::Tuple(Vec::new(), Span::synthetic(0)));

        let where_clauses = self.where_clauses(method);

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
            })
            .collect()
    }

    /// Check if a callable's parameter labels match the given argument labels.
    /// Handles arity with default parameters.
    fn matches_labels(&self, entity: Entity, arg_labels: &[Option<&str>]) -> bool {
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

    /// Build a signature string from a callable's param labels for deduplication.
    /// Two methods with the same name and label signature are considered equivalent overloads.
    /// Includes method name to avoid collisions between different no-arg methods.
    fn label_signature(&self, entity: Entity) -> String {
        let name = self.ctx.get::<Name>(entity)
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

    /// Check if an entity has a conformance to the given protocol.
    fn entity_conforms_to(&self, entity: Entity, protocol: Entity) -> bool {
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

    /// Resolve an AstType to a type entity using ResolveTypePath.
    fn resolve_type_entity(&self, ast_ty: &kestrel_ast_builder::AstType) -> Option<Entity> {
        self.resolve_type_entity_in_context(ast_ty, self.owner)
    }

    fn resolve_type_entity_in_context(&self, ast_ty: &kestrel_ast_builder::AstType, context: Entity) -> Option<Entity> {
        use kestrel_ast_builder::AstType;
        match ast_ty {
            AstType::Named { segments, .. } => {
                let seg_names: Vec<String> =
                    segments.iter().map(|s| s.name.clone()).collect();
                match self.ctx.query(ResolveTypePath {
                    segments: seg_names,
                    context,
                    root: self.root,
                }) {
                    TypeResolution::Found(entity) => Some(entity),
                    TypeResolution::SelfType => self.resolve_self_entity(),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Resolve `Self` to the enclosing type entity.
    /// For extension methods, this is the extension target.
    /// For struct/enum methods, this is the parent struct/enum.
    fn resolve_self_entity(&self) -> Option<Entity> {
        let mut current = Some(self.owner);
        while let Some(entity) = current {
            match self.ctx.get::<NodeKind>(entity) {
                Some(NodeKind::Extension) => {
                    return self.ctx.query(kestrel_name_res::ExtensionTargetEntity {
                        extension: entity,
                        root: self.root,
                    });
                }
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
                    return Some(entity);
                }
                _ => current = self.ctx.parent_of(entity),
            }
        }
        None
    }

    /// Gather protocols from an entity's Conformances, recursively walking inherited protocols.
    fn gather_protocol_conformances(
        &self,
        entity: Entity,
        protocols: &mut Vec<Entity>,
        visited: &mut std::collections::HashSet<Entity>,
    ) {
        let Some(conformances) = self.ctx.get::<Conformances>(entity) else {
            return;
        };

        for item in &conformances.0 {
            let ConformanceItem::Positive(ast_ty, _) = item else { continue };
            let Some(resolved) = self.resolve_type_entity(ast_ty) else { continue };

            if self.ctx.get::<NodeKind>(resolved) != Some(&NodeKind::Protocol) {
                continue;
            }

            if !visited.insert(resolved) {
                continue; // Already visited
            }

            protocols.push(resolved);

            // Walk inherited protocols transitively
            self.gather_protocol_conformances(resolved, protocols, visited);
        }
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
        let bound_protocols = self.collect_param_protocol_bounds(param_entity);
        if bound_protocols.is_empty() {
            return Err(MemberError::NotFound);
        }

        let all_candidates = self.search_protocols_for_member(&bound_protocols, name);

        // For type parameters, include both instance and static methods.
        if all_candidates.is_empty() {
            return Err(MemberError::NotFound);
        }

        // Filter by label/arity if args provided
        let arg_labels: Vec<Option<&str>> = args.iter().map(|a| a.label.as_deref()).collect();
        let matches: Vec<Entity> = all_candidates
            .iter()
            .copied()
            .filter(|&c| self.matches_labels(c, &arg_labels))
            .collect();

        let member = match matches.len() {
            0 => {
                if all_candidates.len() == 1 {
                    all_candidates[0]
                } else {
                    return Err(MemberError::NotFound);
                }
            }
            1 => matches[0],
            _ => return Err(MemberError::Ambiguous(matches)),
        };
        let mut resolution = self.build_member_resolution(member)?;

        // Attach protocol type args from the where clause bound.
        // E.g., for `F: Factory[lang.i64]`, the method's protocol is Factory,
        // and its type args are [lang.i64]. These substitute the protocol's
        // type params (T → i64) in the method's return/param types.
        if let Some(self_entity) = resolution.self_type {
            resolution.protocol_type_args = self.find_protocol_type_args_from_bounds(
                param_entity, self_entity,
            );
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
        let owner = self.ctx.parent_of(param_entity).unwrap_or(self.owner);
        let clauses = self.where_clauses(owner);

        // Direct match: where clause says T: Protocol[Args]
        for clause in &clauses {
            if let WhereClause::Bound { param, protocol, protocol_type_args, .. } = clause {
                if *param == param_entity && *protocol == protocol_entity {
                    return protocol_type_args.clone();
                }
            }
        }

        // Inherited match: where clause says T: ParentProtocol, and
        // ParentProtocol: Protocol[Args] in its conformance list.
        // E.g., T: IntConverter, IntConverter: Converter[i64] → find [i64] for Converter.
        for clause in &clauses {
            if let WhereClause::Bound { param, protocol, .. } = clause {
                if *param == param_entity {
                    if let Some(args) = self.find_inherited_protocol_type_args(*protocol, protocol_entity) {
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
            let ConformanceItem::Positive(ast_ty, _) = item else { continue };
            let Some(resolved) = self.resolve_type_entity(ast_ty) else { continue };
            if resolved == target_protocol {
                // Extract type args from the conformance path
                return Some(extract_protocol_type_args(self.ctx, self.owner, self.root, ast_ty));
            }
            // Recurse into inherited protocols
            if self.ctx.get::<NodeKind>(resolved) == Some(&NodeKind::Protocol) {
                if let Some(args) = self.find_inherited_protocol_type_args(resolved, target_protocol) {
                    return Some(args);
                }
            }
        }
        None
    }

    /// Search a set of protocols (and their extensions) for a member by name.
    /// Handles named members via VisibleChildrenByName and subscripts/inits
    /// by NodeKind when using sentinel names.
    fn search_protocols_for_member(
        &self,
        protocols: &[Entity],
        name: &str,
    ) -> Vec<Entity> {
        let mut all_candidates = Vec::new();
        // Dedup within a protocol (inherited methods) but not across protocols
        // so that same-signature methods from different protocols trigger ambiguity.

        for proto in protocols {
            let mut seen_in_proto = std::collections::HashSet::new();

            // Named members inside the protocol
            let children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                parent: *proto,
                name: name.to_string(),
                context: self.owner,
            });
            for &child in &children {
                let sig = self.label_signature(child);
                if seen_in_proto.insert(sig) {
                    all_candidates.push(child);
                }
            }

            // Default implementations in protocol extensions
            let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                target: *proto,
                root: self.root,
            });
            for ext in &extensions {
                let ext_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                    parent: *ext,
                    name: name.to_string(),
                    context: self.owner,
                });
                for &child in &ext_children {
                    let sig = self.label_signature(child);
                    if seen_in_proto.insert(sig) {
                        all_candidates.push(child);
                    }
                }
            }

            // Subscripts and initializers have no Name — search by NodeKind.
            // Check per-protocol: only search by NodeKind if no named members
            // were found for this protocol (to avoid mixing named + NodeKind results).
            let found_named_in_proto = !seen_in_proto.is_empty();
            if !found_named_in_proto && (name == "(subscript)" || name == "init") {
                let target_kind = if name == "(subscript)" {
                    NodeKind::Subscript
                } else {
                    NodeKind::Initializer
                };
                for &child in self.ctx.children_of(*proto) {
                    if self.ctx.get::<NodeKind>(child) == Some(&target_kind) {
                        all_candidates.push(child);
                    }
                }
                for ext in &extensions {
                    for &child in self.ctx.children_of(*ext) {
                        if self.ctx.get::<NodeKind>(child) == Some(&target_kind) {
                            all_candidates.push(child);
                        }
                    }
                }
            }
        }

        all_candidates
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
            }
            1 => matches[0],
            _ => return Err(MemberError::Ambiguous(matches)),
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
        let bound_protocols = self.collect_assoc_type_protocol_bounds(alias_entity);
        if bound_protocols.is_empty() {
            return Err(MemberError::NotFound);
        }

        let all_candidates = self.search_protocols_for_member(&bound_protocols, name);
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
            _ => return Err(MemberError::Ambiguous(matches)),
        };

        self.build_member_resolution(member)
    }

    /// Collect protocol bounds on an associated type (TypeAlias).
    ///
    /// Searches the TypeAlias's Conformances component, the parent protocol's
    /// where clause, and the owner hierarchy for bounds like `Iter: Iterator`
    /// or `where Item: Equatable`.
    fn collect_assoc_type_protocol_bounds(&self, alias_entity: Entity) -> Vec<Entity> {
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
            self.gather_bounds_from_where_clause(alias_entity, parent, &mut protocols, &mut visited);
        }

        // Walk from owner upward — function/extension where clauses may also
        // constrain associated types. E.g., `func contains() where Item: Equatable`
        let mut current = Some(self.owner);
        while let Some(entity) = current {
            if checked.insert(entity) {
                self.gather_bounds_from_where_clause(alias_entity, entity, &mut protocols, &mut visited);
            }
            current = self.ctx.parent_of(entity);
        }

        // Walk inherited protocols transitively
        let mut i = 0;
        while i < protocols.len() {
            let proto = protocols[i];
            self.gather_protocol_conformances(proto, &mut protocols, &mut visited);
            let proto_extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                target: proto,
                root: self.root,
            });
            for ext in &proto_extensions {
                self.gather_protocol_conformances(*ext, &mut protocols, &mut visited);
            }
            i += 1;
        }

        protocols
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
        let mut current = Some(self.owner);
        while let Some(entity) = current {
            if self.ctx.get::<NodeKind>(entity) == Some(&NodeKind::Extension) {
                // Check if this extension targets our protocol
                let ext_target = self.ctx.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: entity,
                    root: self.root,
                });
                if ext_target == Some(target_protocol) {
                    // Get where clause bounds where the subject is the target protocol (Self)
                    for clause in self.where_clauses(entity) {
                        if let WhereClause::Bound { param, protocol, .. } = clause {
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
        let mut protocols = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut checked = std::collections::HashSet::new();

        // Check param's direct parent (function/method that declares the type param)
        if let Some(parent) = self.ctx.parent_of(param_entity) {
            checked.insert(parent);
            self.gather_bounds_from_where_clause(param_entity, parent, &mut protocols, &mut visited);
        }

        // Walk from owner upward to find extension/protocol where clauses
        // that also constrain this type param. E.g., the method being compiled
        // is inside `extend Array[T] where T: Comparable`, and T is Array's param.
        let mut current = Some(self.owner);
        while let Some(entity) = current {
            if checked.insert(entity) {
                self.gather_bounds_from_where_clause(param_entity, entity, &mut protocols, &mut visited);
            }
            current = self.ctx.parent_of(entity);
        }

        // Walk inherited protocols transitively
        let mut i = 0;
        while i < protocols.len() {
            let proto = protocols[i];
            self.gather_protocol_conformances(proto, &mut protocols, &mut visited);
            let proto_extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
                target: proto,
                root: self.root,
            });
            for ext in &proto_extensions {
                self.gather_protocol_conformances(*ext, &mut protocols, &mut visited);
            }
            i += 1;
        }

        protocols
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

    /// Resolve a type path to a TypeParameter or TypeAlias entity
    /// (for direct equalities like `V = Array[E]` or `Item = Optional[T]`).
    fn resolve_type_param_or_assoc(
        &self,
        ast_ty: &kestrel_ast_builder::AstType,
    ) -> Option<Entity> {
        use kestrel_ast_builder::AstType;
        let AstType::Named { segments, .. } = ast_ty else { return None };
        let all_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        match self.ctx.query(ResolveTypePath {
            segments: all_names,
            context: self.owner,
            root: self.root,
        }) {
            TypeResolution::Found(entity)
                if matches!(
                    self.ctx.get::<NodeKind>(entity),
                    Some(&NodeKind::TypeParameter) | Some(&NodeKind::TypeAlias)
                ) =>
            {
                Some(entity)
            }
            _ => None,
        }
    }

    /// Extract (param_entity, assoc_name) from a type path like `T.Item`.
    fn extract_associated_type_path(
        &self,
        ast_ty: &kestrel_ast_builder::AstType,
    ) -> Option<(Entity, String)> {
        use kestrel_ast_builder::AstType;
        match ast_ty {
            AstType::Named { segments, .. } if segments.len() == 2 => {
                // First segment is the type param, second is the associated type name
                let param_name = &segments[0].name;
                let assoc_name = &segments[1].name;

                // Resolve the param
                match self.ctx.query(ResolveTypePath {
                    segments: vec![param_name.clone()],
                    context: self.owner,
                    root: self.root,
                }) {
                    TypeResolution::Found(entity) => {
                        Some((entity, assoc_name.clone()))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

