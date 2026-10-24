//! The front end's [`EntailLayer`]: world facts for the `kestrel-entail`
//! kernel, read from the name-res / hir-lower queries.
//!
//! One layer serves both front-end callers (bidi design §7.2):
//! - the **analyzer** (`conformance::type_satisfies`), which converts `HirTy`
//!   with [`FrontEndLayer::hir_to_ety`]; and
//! - the **solver**, which converts `TyVar`s with `solver::solver_ety`, where
//!   an open var becomes [`ETy::Var`] and so every question touching it is
//!   `Unknown` instead of the old `reify_tv` → `HirTy::Error` → permit (N1).
//!
//! Where clauses become kernel [`Goal`]s here, *including equality clauses*,
//! which every evaluator used to skip as satisfied (N2).

use std::cell::RefCell;

use kestrel_ast_builder::{NodeKind, TypeParams};
use kestrel_entail::{ETy, EntailLayer, Goal, Source, Supply, Tri};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::builtin::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::LowerExtensionTargetTypeArgs;
use kestrel_name_res::{
    ConformingProtocolInstantiations, ConformingProtocols, ExtensionTargetEntity, ResolveBuiltin,
};
use kestrel_span::Span;

use crate::resolve::{WhereClause, WhereSubject};
use crate::where_clauses::WhereClausesOf;

pub(crate) struct FrontEndLayer<'q, 'c> {
    ctx: &'q QueryContext<'c>,
    root: Entity,
    /// Originals of [`ETy::Abstract`] handles (functions, opaques, aliases,
    /// abstract `Self`), so builtin classifiers can still see them.
    abstracts: RefCell<Vec<HirTy>>,
}

impl<'q, 'c> FrontEndLayer<'q, 'c> {
    pub(crate) fn new(ctx: &'q QueryContext<'c>, root: Entity) -> Self {
        Self {
            ctx,
            root,
            abstracts: RefCell::new(Vec::new()),
        }
    }

    /// Register a layer-specific type and get its [`ETy::Abstract`] handle.
    pub(crate) fn abstract_of(&self, ty: HirTy) -> ETy {
        let mut table = self.abstracts.borrow_mut();
        table.push(ty);
        ETy::Abstract((table.len() - 1) as u32)
    }

    /// `HirTy` → kernel type. `Infer` is an open var (Unknown); `Error` is
    /// poison (Holds).
    pub(crate) fn hir_to_ety(&self, ty: &HirTy) -> ETy {
        match ty {
            HirTy::Struct { entity, args, .. }
            | HirTy::Enum { entity, args, .. }
            | HirTy::Protocol { entity, args, .. } => ETy::Nominal {
                entity: *entity,
                args: args.iter().map(|a| self.hir_to_ety(a)).collect(),
            },
            HirTy::Tuple(elems, _) => ETy::Tuple(elems.iter().map(|a| self.hir_to_ety(a)).collect()),
            HirTy::Never(_) => ETy::Never,
            HirTy::Param(e, _) => ETy::Param(*e),
            HirTy::AssocProjection { base, assoc, .. } => ETy::Proj {
                base: Box::new(self.hir_to_ety(base)),
                assoc: *assoc,
            },
            HirTy::Ref { inner, mutating, .. } => ETy::Ref {
                inner: Box::new(self.hir_to_ety(inner)),
                mutating: *mutating,
            },
            HirTy::Infer(_) => ETy::Var,
            HirTy::Error(_) => ETy::Poison,
            HirTy::Function { .. }
            | HirTy::AliasUse { .. }
            | HirTy::SelfType(..)
            | HirTy::Opaque { .. } => self.abstract_of(ty.clone()),
        }
    }

    /// Kernel type → `HirTy`, for the builtin classifiers (copy semantics,
    /// staticness). Only called on types without `Var`.
    fn ety_to_hir(&self, ty: &ETy) -> HirTy {
        let span = Span::synthetic(0);
        match ty {
            ETy::Nominal { entity, args } => {
                let args = args.iter().map(|a| self.ety_to_hir(a)).collect();
                match self.ctx.get::<NodeKind>(*entity) {
                    Some(NodeKind::Enum) => HirTy::Enum {
                        entity: *entity,
                        args,
                        span,
                    },
                    Some(NodeKind::Protocol) => HirTy::Protocol {
                        entity: *entity,
                        args,
                        span,
                    },
                    _ => HirTy::Struct {
                        entity: *entity,
                        args,
                        span,
                    },
                }
            },
            ETy::Tuple(elems) => HirTy::Tuple(elems.iter().map(|a| self.ety_to_hir(a)).collect(), span),
            ETy::Never => HirTy::Never(span),
            ETy::Param(e) => HirTy::Param(*e, span),
            ETy::Proj { base, assoc } => HirTy::AssocProjection {
                base: Box::new(self.ety_to_hir(base)),
                assoc: *assoc,
                span,
            },
            ETy::Ref { inner, mutating } => HirTy::Ref {
                inner: Box::new(self.ety_to_hir(inner)),
                mutating: *mutating,
                span,
            },
            ETy::Abstract(i) => self.abstracts.borrow()[*i as usize].clone(),
            ETy::SelfTy | ETy::Var | ETy::LitVar(_) | ETy::Poison => HirTy::Error(span),
        }
    }

    /// An extension's own target-arg pattern (`[T, E]` in
    /// `extend Result[T, E]`).
    pub(crate) fn extension_target(&self, extension: Entity) -> Vec<ETy> {
        self.ctx
            .query(LowerExtensionTargetTypeArgs {
                extension,
                root: self.root,
            })
            .unwrap_or_default()
            .iter()
            .map(|t| self.hir_to_ety(t))
            .collect()
    }

    /// An extension's where clauses as kernel goals over its params and
    /// `Self`. A subject spelled with the target protocol's own name
    /// (`extend Producer where Producer: Q`) means `Self`.
    pub(crate) fn extension_clauses(&self, extension: Entity) -> Vec<Goal> {
        let target = self.ctx.query(ExtensionTargetEntity {
            extension,
            root: self.root,
        });
        self.ctx
            .query(WhereClausesOf {
                entity: extension,
                root: self.root,
            })
            .iter()
            .map(|clause| match clause {
                WhereClause::Bound {
                    subject, protocol, ..
                } => Goal::Conforms {
                    ty: self.subject_ety(subject, target),
                    protocol: *protocol,
                },
                WhereClause::Equality { subject, rhs } => {
                    Goal::Equal(self.subject_ety(subject, target), self.hir_to_ety(rhs))
                },
            })
            .collect()
    }

    fn subject_ety(&self, subject: &WhereSubject, target: Option<Entity>) -> ETy {
        match subject {
            WhereSubject::SelfType => ETy::SelfTy,
            WhereSubject::Param(p) if Some(*p) == target => ETy::SelfTy,
            // A bare associated-type name in a protocol extension's clause
            // (`extend Source where Item: P`) is `Self.Item`, not a param.
            WhereSubject::Param(p)
                if self.ctx.get::<NodeKind>(*p) == Some(&NodeKind::TypeAlias) =>
            {
                ETy::Proj {
                    base: Box::new(ETy::SelfTy),
                    assoc: *p,
                }
            },
            WhereSubject::Param(p) => ETy::Param(*p),
            WhereSubject::Projection { base, assoc } => ETy::Proj {
                base: Box::new(self.subject_ety(base, target)),
                assoc: *assoc,
            },
        }
    }

    fn builtin(&self, builtin: Builtin) -> Option<Entity> {
        self.ctx.query(ResolveBuiltin {
            builtin,
            root: self.root,
        })
    }

    /// Is this an abstract position for the copy/static classifiers (they
    /// permit those: per-instantiation precision is the solver's job)?
    fn abstract_for_classifier(&self, ty: &ETy) -> bool {
        match ty {
            ETy::Param(_) | ETy::SelfTy | ETy::Proj { .. } => true,
            ETy::Abstract(i) => matches!(
                self.abstracts.borrow()[*i as usize],
                HirTy::SelfType(..)
                    | HirTy::AssocProjection { .. }
                    | HirTy::Opaque { .. }
                    | HirTy::Error(..)
            ),
            _ => false,
        }
    }

    /// Who supplies `entity[..]: protocol`.
    fn nominal_supply(&self, entity: Entity, protocol: Entity) -> Supply {
        // Must at least *declare* the conformance (closure-aware: inheritance,
        // extension-added, refinement). A plain non-conformer fails here.
        if !self
            .ctx
            .query(ConformingProtocols {
                entity,
                root: self.root,
            })
            .contains(&protocol)
        {
            return Supply::Sources(Vec::new());
        }
        // A protocol-typed receiver is an existential: nothing concrete to
        // disprove.
        if self.ctx.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) {
            return Supply::Decided(Tri::Holds);
        }
        let insts = self.ctx.query(ConformingProtocolInstantiations {
            entity,
            root: self.root,
        });
        let mut sources = Vec::new();
        for (proto, source, _args) in &insts {
            if *proto != protocol {
                continue;
            }
            match self.ctx.get::<NodeKind>(*source) {
                Some(NodeKind::Extension) => {
                    // A blanket `extend Parent: P` supplies P through Parent.
                    if let Some(target) = self.ctx.query(ExtensionTargetEntity {
                        extension: *source,
                        root: self.root,
                    }) && self.ctx.get::<NodeKind>(target) == Some(&NodeKind::Protocol)
                    {
                        sources.push(Source::Refines(target));
                        continue;
                    }
                    sources.push(Source::Extension {
                        target_args: self.extension_target(*source),
                        clauses: self.extension_clauses(*source),
                    });
                },
                // Refinement: `protocol Comparable: Equatable`.
                Some(NodeKind::Protocol) => sources.push(Source::Refines(*source)),
                // The type's own declaration header.
                _ => sources.push(Source::Unconditional),
            }
        }
        // Declared, but through a path the instantiation query does not
        // enumerate: the declaration is the answer.
        if sources.is_empty() {
            return Supply::Decided(Tri::Holds);
        }
        Supply::Sources(sources)
    }

    /// `param: protocol` from the param's declared bounds (its owner's where
    /// clauses, refinement-closed). Not entailed is `Unknown`, not `Fails`:
    /// the asking site's own context is not threaded here yet, so absence of
    /// a declared bound proves nothing.
    fn param_supply(&self, param: Entity, protocol: Entity) -> Tri {
        let bound = WhereClause::Bound {
            subject: WhereSubject::Param(param),
            protocol,
            protocol_type_args: Vec::new(),
        };
        if crate::entailment::constraint_entailed_by(self.ctx, self.root, &bound, &[]) {
            Tri::Holds
        } else {
            Tri::Unknown
        }
    }
}

impl EntailLayer for FrontEndLayer<'_, '_> {
    fn supply(&self, ty: &ETy, protocol: Entity) -> Supply {
        // Static is structural, never declared.
        if self.builtin(Builtin::Static) == Some(protocol) {
            if self.abstract_for_classifier(ty) {
                return Supply::Decided(Tri::Holds);
            }
            let hir = self.ety_to_hir(ty);
            return Supply::Decided(Tri::from_bool(kestrel_semantics::hir_type_is_static(
                self.ctx, &hir, self.root, self.root,
            )));
        }
        // Copyable / Cloneable are structural too (copy semantics), never
        // declared: answer with the shared classifier so there is one story.
        let copyable = self.builtin(Builtin::Copyable) == Some(protocol);
        let cloneable = self.builtin(Builtin::Cloneable) == Some(protocol);
        if copyable || cloneable {
            if self.abstract_for_classifier(ty) {
                return Supply::Decided(Tri::Holds);
            }
            let hir = self.ety_to_hir(ty);
            let sem = kestrel_semantics::hir_type_copy_semantics(self.ctx, &hir, self.root, self.root);
            let ok = if cloneable {
                sem == kestrel_semantics::CopySemantics::Cloneable
            } else {
                sem != kestrel_semantics::CopySemantics::NotCopyable
            };
            return Supply::Decided(Tri::from_bool(ok));
        }
        match ty {
            ETy::Nominal { entity, .. } => self.nominal_supply(*entity, protocol),
            // A ref conforms through `extend &T: P` / `extend &mutating T: P`
            // on the synthetic `lang.&` entities (args = [pointee]). No
            // declaring extension → no conformance.
            ETy::Ref { mutating, .. } => {
                let name = if *mutating { "&mutating" } else { "&" };
                match kestrel_name_res::extensions::resolve_lang_child(self.ctx, self.root, name) {
                    Some(amp) => self.nominal_supply(amp, protocol),
                    None => Supply::Sources(Vec::new()),
                }
            },
            // `()` and `!` conform through their synthetic `lang` entities.
            ETy::Tuple(elems) if elems.is_empty() => self.lang_supply("()", protocol),
            ETy::Never => self.lang_supply("!", protocol),
            ETy::Param(p) => Supply::Decided(self.param_supply(*p, protocol)),
            // Non-empty tuples, functions, opaques, unreduced aliases and
            // projections, abstract `Self`: no declaration to evaluate here.
            _ => Supply::Decided(Tri::Holds),
        }
    }

    fn assoc_binding(&self, base: &ETy, assoc: Entity) -> Option<ETy> {
        let ETy::Nominal { entity, args } = base else {
            return None;
        };
        let name = &self.ctx.get::<kestrel_ast_builder::Name>(assoc)?.0;
        let extensions = self.ctx.query(kestrel_name_res::ExtensionsFor {
            target: *entity,
            root: self.root,
        });
        // The type's own params bind positionally to the base's args.
        let own_params: Vec<(Entity, ETy)> = self
            .ctx
            .get::<TypeParams>(*entity)
            .map(|tp| tp.0.iter().copied().zip(args.iter().cloned()).collect())
            .unwrap_or_default();
        let mut found: Vec<ETy> = Vec::new();
        if let Some(t) = self.assoc_binding_in(*entity, name, assoc) {
            found.push(kestrel_entail::subst(&t, &own_params, base));
        }
        for &ext in extensions.iter() {
            let Some(t) = self.assoc_binding_in(ext, name, assoc) else {
                continue;
            };
            // An extension's params bind through its target pattern.
            let target = self.extension_target(ext);
            let binds: Vec<(Entity, ETy)> = target
                .iter()
                .zip(args.iter())
                .filter_map(|(p, a)| match p {
                    ETy::Param(e) => Some((*e, a.clone())),
                    _ => None,
                })
                .collect();
            found.push(kestrel_entail::subst(&t, &binds, base));
        }
        // Two protocols can declare an associated type of the same name and
        // this lookup is by name: more than one candidate means "cannot
        // tell", never a guess.
        (found.len() == 1).then(|| found.remove(0))
    }
}

impl FrontEndLayer<'_, '_> {
    fn lang_supply(&self, name: &str, protocol: Entity) -> Supply {
        match kestrel_name_res::extensions::resolve_lang_child(self.ctx, self.root, name) {
            Some(e) => self.nominal_supply(e, protocol),
            None => Supply::Decided(Tri::Holds),
        }
    }

    /// A `type <name> = …` child of `owner` with a lowerable annotation that
    /// witnesses `assoc`: unqualified, or qualified by the protocol that
    /// declares `assoc` (`type Addable.Output = Int8` witnesses
    /// `Addable.Output`, not `Equal.Output`).
    fn assoc_binding_in(&self, owner: Entity, name: &str, assoc: Entity) -> Option<ETy> {
        let declaring = self.ctx.parent_of(assoc);
        self.ctx.children_of(owner).iter().copied().find_map(|child| {
            let named = self.ctx.get::<NodeKind>(child) == Some(&NodeKind::TypeAlias)
                && self
                    .ctx
                    .get::<kestrel_ast_builder::Name>(child)
                    .is_some_and(|n| n.0 == name);
            if !named || !self.qualifier_matches(child, declaring) {
                return None;
            }
            self.ctx
                .query(kestrel_hir_lower::LowerTypeAnnotation {
                    entity: child,
                    root: self.root,
                })
                .map(|t| self.hir_to_ety(&t))
        })
    }

    /// Does alias `child`'s qualifier (if any) name `protocol`?
    fn qualifier_matches(&self, child: Entity, protocol: Option<Entity>) -> bool {
        let Some(target) = self.ctx.get::<kestrel_ast_builder::QualifiedTarget>(child) else {
            return true;
        };
        let kestrel_ast::AstType::Named { segments, .. } = &target.0 else {
            return true;
        };
        let resolved = self.ctx.query(kestrel_name_res::resolve_type::ResolveTypePath {
            segments: segments.iter().map(|s| s.name.clone()).collect(),
            context: child,
            root: self.root,
        });
        match resolved {
            kestrel_name_res::resolve_type::TypeResolution::Found(e) => Some(e) == protocol,
            _ => true,
        }
    }
}
