//! Type lowering: AstType → HirTy.
//!
//! Resolves type paths to entities and expands sugar types
//! (Array, Optional, Dictionary, Result) into Named types.

use kestrel_ast::AstType;
use kestrel_hir::ty::HirTy;
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

impl LowerCtx<'_> {
    /// Lower an AST type to an HIR type.
    pub fn lower_type(&mut self, ty: &AstType) -> HirTy {
        match ty {
            AstType::Named { segments, span } => {
                let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
                let result = self.ctx.query(ResolveTypePath {
                    segments: seg_names,
                    context: self.owner,
                    root: self.root,
                });

                match result {
                    TypeResolution::Found(entity) => {
                        // Check if entity is a type parameter
                        if self.ctx.get::<kestrel_ast_builder::NodeKind>(entity)
                            == Some(&kestrel_ast_builder::NodeKind::TypeParameter)
                        {
                            // Type params shouldn't have user-provided args
                            return HirTy::Param(entity, span.clone());
                        }

                        // Lower type arguments from all segments
                        let args: Vec<HirTy> = segments
                            .iter()
                            .flat_map(|s| s.type_args.iter())
                            .map(|a| self.lower_type(a))
                            .collect();

                        HirTy::Named {
                            entity,
                            args,
                            span: span.clone(),
                        }
                    }
                    TypeResolution::SelfType => {
                        // Find the enclosing type to resolve Self
                        if let Some(self_entity) = self.find_self_type() {
                            HirTy::Named {
                                entity: self_entity,
                                args: Vec::new(),
                                span: span.clone(),
                            }
                        } else {
                            HirTy::Error(span.clone())
                        }
                    }
                    TypeResolution::NotFound(_) | TypeResolution::NotAType(_) => {
                        HirTy::Error(span.clone())
                    }
                }
            }

            AstType::Tuple(types, span) => {
                let lowered: Vec<HirTy> = types.iter().map(|t| self.lower_type(t)).collect();
                HirTy::Tuple(lowered, span.clone())
            }

            AstType::Function {
                params,
                return_type,
                span,
            } => {
                let lowered_params: Vec<HirTy> = params.iter().map(|t| self.lower_type(t)).collect();
                let lowered_ret = Box::new(self.lower_type(return_type));
                HirTy::Function {
                    params: lowered_params,
                    ret: lowered_ret,
                    span: span.clone(),
                }
            }

            // Sugar types → resolve standard library entity + Named
            AstType::Array(elem, span) => self.lower_sugar_type("Array", &[elem], span),
            AstType::Optional(inner, span) => self.lower_sugar_type("Optional", &[inner], span),
            AstType::Dictionary(key, val, span) => {
                self.lower_sugar_type("Dictionary", &[key, val], span)
            }
            AstType::Result { ok, err, span } => {
                self.lower_sugar_type("Result", &[ok, err], span)
            }
            AstType::Unit(span) => HirTy::Tuple(Vec::new(), span.clone()),
            AstType::Never(span) => {
                // Resolve Never as a named type
                if let Some(entity) = self.resolve_std_type("Never") {
                    HirTy::Named {
                        entity,
                        args: Vec::new(),
                        span: span.clone(),
                    }
                } else {
                    HirTy::Error(span.clone())
                }
            }
            AstType::Inferred(span) => HirTy::Infer(span.clone()),
        }
    }

    /// Lower a sugar type (Array, Optional, etc.) by resolving the std type entity.
    fn lower_sugar_type(&mut self, name: &str, type_args: &[&Box<AstType>], span: &Span) -> HirTy {
        let lowered_args: Vec<HirTy> = type_args.iter().map(|t| self.lower_type(t)).collect();

        if let Some(entity) = self.resolve_std_type(name) {
            HirTy::Named {
                entity,
                args: lowered_args,
                span: span.clone(),
            }
        } else {
            HirTy::Error(span.clone())
        }
    }

    /// Find the enclosing type entity for Self resolution.
    /// Walks up from owner to find the nearest Struct/Enum/Protocol.
    fn find_self_type(&self) -> Option<kestrel_hecs::Entity> {
        use kestrel_ast_builder::NodeKind;
        let mut current = Some(self.owner);
        while let Some(entity) = current {
            match self.ctx.get::<NodeKind>(entity) {
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
                    return Some(entity);
                }
                Some(NodeKind::Extension) => {
                    // For extensions, resolve the extension target type
                    // The extension target entity should be resolved elsewhere
                    return Some(entity);
                }
                _ => {
                    current = self.ctx.parent_of(entity);
                }
            }
        }
        None
    }
}
