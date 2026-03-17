//! Pattern lowering: AstPat → HirPat.
//!
//! Allocates local variable slots for pattern bindings and resolves
//! enum/struct pattern names to entities where possible.

use kestrel_ast::ast_body::*;
use kestrel_hir::body::*;
use kestrel_name_res::{ResolveValuePath, TypeResolution, ResolveTypePath, ValueResolution};
use kestrel_span2::Span;

use crate::ctx::LowerCtx;

impl LowerCtx<'_> {
    /// Lower an AST pattern to an HIR pattern.
    pub fn lower_pat(&mut self, body: &AstBody, id: PatId) -> HirPatId {
        let pat = &body.pats[id];
        match pat {
            AstPat::Wildcard { span } => self.alloc_pat(HirPat::Wildcard {
                span: span.clone(),
            }),

            AstPat::Binding { is_mut, name, span } => {
                let local = self.define_local(name, *is_mut, span.clone());
                self.alloc_pat(HirPat::Binding {
                    local,
                    span: span.clone(),
                })
            }

            AstPat::Tuple { elements, span } => {
                let lowered: Vec<HirPatId> =
                    elements.iter().map(|&id| self.lower_pat(body, id)).collect();
                self.alloc_pat(HirPat::Tuple {
                    elements: lowered,
                    span: span.clone(),
                })
            }

            AstPat::Literal { kind, span } => {
                let value = lower_lit_pat(kind);
                self.alloc_pat(HirPat::Literal {
                    value,
                    span: span.clone(),
                })
            }

            AstPat::Range {
                start,
                end,
                inclusive,
                span,
            } => self.alloc_pat(HirPat::Range {
                start: start.as_ref().map(lower_lit_pat),
                end: end.as_ref().map(lower_lit_pat),
                inclusive: *inclusive,
                span: span.clone(),
            }),

            AstPat::Enum {
                case_name,
                args,
                span,
            } => self.lower_enum_pat(body, case_name, args, span),

            AstPat::Struct {
                name,
                fields,
                has_rest,
                span,
            } => self.lower_struct_pat(body, name, fields, *has_rest, span),

            AstPat::Array { span, .. } => {
                // Array patterns not yet supported in HirPat
                self.alloc_pat(HirPat::Error {
                    span: span.clone(),
                })
            }

            AstPat::At {
                is_mut,
                name,
                subpattern,
                span,
            } => {
                // `name @ subpattern` — bind the whole value to name, also match subpattern
                let local = self.define_local(name, *is_mut, span.clone());
                let lowered_sub = self.lower_pat(body, *subpattern);
                self.alloc_pat(HirPat::At {
                    binding: local,
                    subpattern: lowered_sub,
                    span: span.clone(),
                })
            }

            AstPat::Or { alternatives, span } => {
                let lowered: Vec<HirPatId> = alternatives
                    .iter()
                    .map(|&id| self.lower_pat(body, id))
                    .collect();
                self.alloc_pat(HirPat::Or {
                    alternatives: lowered,
                    span: span.clone(),
                })
            }

            AstPat::Rest { span } => {
                // Rest should be absorbed by parent — standalone is an error
                self.alloc_pat(HirPat::Error {
                    span: span.clone(),
                })
            }

            AstPat::Error { span } => self.alloc_pat(HirPat::Error {
                span: span.clone(),
            }),
        }
    }

    /// Lower an enum pattern. Try to resolve the case name to an entity.
    fn lower_enum_pat(
        &mut self,
        body: &AstBody,
        case_name: &str,
        args: &[EnumPatArg],
        span: &Span,
    ) -> HirPatId {
        let lowered_args: Vec<HirPatArg> = args
            .iter()
            .map(|arg| HirPatArg {
                label: arg.label.clone(),
                pattern: self.lower_pat(body, arg.pattern),
            })
            .collect();

        // Try to resolve as a qualified enum case (e.g. "MyEnum.caseA" if multi-segment,
        // or just "CaseName" if it's a known enum case in scope)
        let result = self.ctx.query(ResolveValuePath {
            segments: vec![case_name.to_string()],
            context: self.owner,
            root: self.root,
        });

        match result {
            ValueResolution::Def(entity) => {
                // Check if it's actually an enum case
                if self.ctx.get::<kestrel_ast_builder::NodeKind>(entity)
                    == Some(&kestrel_ast_builder::NodeKind::EnumCase)
                {
                    self.alloc_pat(HirPat::Variant {
                        entity,
                        args: lowered_args,
                        span: span.clone(),
                    })
                } else {
                    // Found something but it's not an enum case — treat as implicit
                    self.alloc_pat(HirPat::ImplicitVariant {
                        name: case_name.to_string(),
                        args: lowered_args,
                        span: span.clone(),
                    })
                }
            }
            _ => {
                // Not found or ambiguous — leave as implicit for type inference
                self.alloc_pat(HirPat::ImplicitVariant {
                    name: case_name.to_string(),
                    args: lowered_args,
                    span: span.clone(),
                })
            }
        }
    }

    /// Lower a struct pattern. Resolve the struct name to an entity.
    fn lower_struct_pat(
        &mut self,
        body: &AstBody,
        name: &str,
        fields: &[StructPatField],
        has_rest: bool,
        span: &Span,
    ) -> HirPatId {
        let lowered_fields: Vec<HirStructPatField> = fields
            .iter()
            .map(|f| HirStructPatField {
                field_name: f.field_name.clone(),
                pattern: f.pattern.map(|id| self.lower_pat(body, id)),
            })
            .collect();

        // Try to resolve struct name
        let result = self.ctx.query(ResolveTypePath {
            segments: vec![name.to_string()],
            context: self.owner,
            root: self.root,
        });

        match result {
            TypeResolution::Found(entity) => self.alloc_pat(HirPat::Struct {
                entity,
                fields: lowered_fields,
                has_rest,
                span: span.clone(),
            }),
            _ => self.alloc_pat(HirPat::Error {
                span: span.clone(),
            }),
        }
    }
}

/// Convert a literal pattern kind to an HIR literal.
fn lower_lit_pat(kind: &LitPatKind) -> HirLiteral {
    match kind {
        LitPatKind::Integer(s) => HirLiteral::Integer(parse_int(s)),
        LitPatKind::Float(s) => HirLiteral::Float(parse_float(s)),
        LitPatKind::String(s) => HirLiteral::String(s.clone()),
        LitPatKind::Bool(b) => HirLiteral::Bool(*b),
        LitPatKind::Char(s) => HirLiteral::Char(parse_char(s)),
    }
}

/// Parse an integer literal string to i64.
pub(crate) fn parse_int(s: &str) -> i64 {
    let s = s.replace('_', "");
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).unwrap_or(0)
    } else if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        i64::from_str_radix(oct, 8).unwrap_or(0)
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        i64::from_str_radix(bin, 2).unwrap_or(0)
    } else {
        s.parse().unwrap_or(0)
    }
}

/// Parse a float literal string to f64.
pub(crate) fn parse_float(s: &str) -> f64 {
    s.replace('_', "").parse().unwrap_or(0.0)
}

/// Parse a char literal string to a unicode scalar value.
pub(crate) fn parse_char(s: &str) -> u32 {
    // Strip surrounding quotes if present
    let inner = s.trim_matches('\'');
    // Handle escape sequences
    if inner.starts_with('\\') {
        match inner.chars().nth(1) {
            Some('n') => '\n' as u32,
            Some('r') => '\r' as u32,
            Some('t') => '\t' as u32,
            Some('\\') => '\\' as u32,
            Some('\'') => '\'' as u32,
            Some('0') => 0,
            Some('u') => {
                // \u{XXXX}
                let hex = inner.trim_start_matches("\\u{").trim_end_matches('}');
                u32::from_str_radix(hex, 16).unwrap_or(0)
            }
            _ => inner.chars().next().map(|c| c as u32).unwrap_or(0),
        }
    } else {
        inner.chars().next().map(|c| c as u32).unwrap_or(0)
    }
}
