//! Type resolver trait and world implementation.
//!
//! `TypeResolver` abstracts world queries for testability. The solver
//! depends on this trait, not on concrete `QueryContext`. `WorldResolver`
//! is the real implementation; tests can provide mocks.

use kestrel_ast_builder::{
    Callable, Conformances, ConformanceItem, Gettable, Name, NodeKind, Settable, Static,
    TypeAnnotation, WhereClause as AstWhereClause, WhereConstraint,
};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::Builtin;
use kestrel_name_res::{ResolveBuiltin, ResolveTypePath, TypeResolution};

use crate::ty::TyKind;

/// Result of resolving a member on a type.
#[derive(Clone, Debug)]
pub struct MemberResolution {
    /// The resolved entity (function, field, getter, etc.)
    pub entity: Entity,
    /// Type parameters of the member (to be instantiated with fresh TyVars).
    pub type_params: Vec<Entity>,
    /// Parameter types (with type param placeholders as `TyKind::Param`).
    pub param_types: Vec<ParamInfo>,
    /// Return type (with type param placeholders).
    pub return_type: TyKind,
    /// Where clauses on this member.
    pub where_clauses: Vec<WhereClause>,
    /// What kind of member this is.
    pub kind: MemberKind,
}

/// Info about a member's parameter, for overload resolution.
#[derive(Clone, Debug)]
pub struct ParamInfo {
    pub label: Option<String>,
    pub ty: TyKind,
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
    pub resolved: TyKind,
}

/// Where clause on a declaration.
#[derive(Clone, Debug)]
pub enum WhereClause {
    /// `T: Protocol`
    Bound { param: Entity, protocol: Entity },
    /// `T.Item = SomeType`
    TypeEquality {
        param: Entity,
        assoc_name: String,
        rhs: TyKind,
    },
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
        let TyKind::Named { entity, .. } = receiver_ty else {
            return Err(MemberError::NotFound);
        };

        // Search direct children by name
        let candidates = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
            parent: *entity,
            name: name.to_string(),
            context: self.owner,
        });

        // Also search extensions
        let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
            target: *entity,
            root: self.root,
        });
        let mut all_candidates = candidates;
        for ext in &extensions {
            let ext_children = self.ctx.query(kestrel_name_res::VisibleChildrenByName {
                parent: *ext,
                name: name.to_string(),
                context: self.owner,
            });
            all_candidates.extend(ext_children);
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
            _ => return Err(MemberError::Ambiguous(matches)),
        };

        self.build_member_resolution(member)
    }

    fn conforms_to(&self, ty: &TyKind, protocol: Entity) -> bool {
        let TyKind::Named { entity, .. } = ty else {
            return false;
        };

        // Check direct conformances on the type entity
        if self.entity_conforms_to(*entity, protocol) {
            return true;
        }

        // Check conformances from extensions
        if let Ok(extensions) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.ctx.query(kestrel_name_res::ExtensionsFor {
                target: *entity,
                root: self.root,
            })
        })) {
            for ext in &extensions {
                if self.entity_conforms_to(*ext, protocol) {
                    return true;
                }
            }
        }

        false
    }

    fn resolve_associated_type(
        &self,
        container: &TyKind,
        name: &str,
    ) -> Option<AssociatedTypeResolution> {
        let TyKind::Named { entity, .. } = container else {
            return None;
        };

        // Search children of the type for a TypeAlias with matching name
        for &child in self.ctx.children_of(*entity) {
            if self.ctx.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && self
                    .ctx
                    .get::<Name>(child)
                    .is_some_and(|n| n.0 == name)
            {
                // Found the associated type — read its TypeAnnotation
                if let Some(type_ann) = self.ctx.get::<TypeAnnotation>(child) {
                    let hir_ty = crate::generate::lower_ast_type(
                        self.ctx,
                        self.owner,
                        self.root,
                        &type_ann.0,
                    );
                    let resolved = hir_ty_to_tykind(&hir_ty);
                    return Some(AssociatedTypeResolution { resolved });
                }
            }
        }

        None
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
        let from_value_protocol = self.builtin(Builtin::FromValue)?;

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
                    let Some(param) = self.resolve_type_entity(subject) else {
                        continue;
                    };

                    // Resolve each protocol
                    for protocol_ty in protocols {
                        if let Some(protocol) = self.resolve_type_entity(protocol_ty) {
                            result.push(WhereClause::Bound { param, protocol });
                        }
                    }
                }
                WhereConstraint::Equality { lhs, rhs, .. } => {
                    // Type equality: resolve both sides
                    // lhs is typically T.Assoc, rhs is a concrete type
                    // For now, we need to extract the param and assoc name from lhs
                    if let Some((param, assoc_name)) = self.extract_associated_type_path(lhs) {
                        let rhs_hir = crate::generate::lower_ast_type(
                            self.ctx,
                            self.owner,
                            self.root,
                            rhs,
                        );
                        let rhs_kind = hir_ty_to_tykind(&rhs_hir);
                        result.push(WhereClause::TypeEquality {
                            param,
                            assoc_name,
                            rhs: rhs_kind,
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
}

impl WorldResolver<'_> {
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

        // Build parameter types from Callable component
        let param_types: Vec<ParamInfo> = if let Some(callable) = self.ctx.get::<Callable>(member)
        {
            callable
                .params
                .iter()
                .map(|p| {
                    let ty = if let Some(ast_ty) = &p.ty {
                        let hir_ty = crate::generate::lower_ast_type(
                            self.ctx,
                            self.owner,
                            self.root,
                            ast_ty,
                        );
                        hir_ty_to_tykind(&hir_ty)
                    } else {
                        TyKind::Error
                    };
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
                if let Some(type_ann) = self.ctx.get::<TypeAnnotation>(member) {
                    let hir_ty = crate::generate::lower_ast_type(
                        self.ctx,
                        self.owner,
                        self.root,
                        &type_ann.0,
                    );
                    hir_ty_to_tykind(&hir_ty)
                } else {
                    TyKind::Error
                }
            }
            _ => {
                // Methods/inits/subscripts: return type from annotation
                if let Some(type_ann) = self.ctx.get::<TypeAnnotation>(member) {
                    let hir_ty = crate::generate::lower_ast_type(
                        self.ctx,
                        self.owner,
                        self.root,
                        &type_ann.0,
                    );
                    hir_ty_to_tykind(&hir_ty)
                } else {
                    // No return annotation — could be void or init (returns Self)
                    TyKind::Tuple(vec![]) // Unit/void
                }
            }
        };

        // Get where clauses
        let where_clauses = self.where_clauses(member);

        Ok(MemberResolution {
            entity: member,
            type_params,
            param_types,
            return_type,
            where_clauses,
            kind: member_kind,
        })
    }

    /// Check if a callable's parameter labels match the given argument labels.
    /// Handles arity with default parameters.
    fn matches_labels(&self, entity: Entity, arg_labels: &[Option<&str>]) -> bool {
        let Some(callable) = self.ctx.get::<Callable>(entity) else {
            // Non-callable members (fields, properties) match if no args
            return arg_labels.is_empty();
        };

        let params = &callable.params;
        let required_count = params.iter().filter(|p| !p.has_default).count();

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
        use kestrel_ast_builder::AstType;
        match ast_ty {
            AstType::Named { segments, .. } => {
                let seg_names: Vec<String> =
                    segments.iter().map(|s| s.name.clone()).collect();
                match self.ctx.query(ResolveTypePath {
                    segments: seg_names,
                    context: self.owner,
                    root: self.root,
                }) {
                    TypeResolution::Found(entity) => Some(entity),
                    _ => None,
                }
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

/// Convert HirTy to TyKind (no TyVar allocation — uses Param for type params).
/// Used by the resolver to return type structures without needing InferCtx.
fn hir_ty_to_tykind(ty: &kestrel_hir::ty::HirTy) -> TyKind {
    use kestrel_hir::ty::HirTy;
    match ty {
        HirTy::Named { entity, args: _, .. } => {
            // For resolver-returned types, we can't allocate TyVars.
            // We use a placeholder approach: return TyKind::Named with empty args
            // for non-generic types, and TyKind::Param for type parameters.
            // The args are converted recursively but won't have valid TyVar indices.
            // This is OK because the solver's kind_to_tyvar re-allocates everything.
            TyKind::Named {
                entity: *entity,
                args: vec![], // Args handled by the caller during instantiation
            }
        }
        HirTy::Param(entity, _) => TyKind::Param { entity: *entity },
        HirTy::Tuple(_, _) => TyKind::Tuple(vec![]),
        HirTy::Function { .. } => TyKind::Function {
            params: vec![],
            ret: crate::ty::TyVar(0),
        },
        HirTy::Infer(_) => TyKind::Error,
        HirTy::Error(_) => TyKind::Error,
    }
}
