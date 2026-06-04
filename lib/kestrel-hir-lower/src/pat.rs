//! Pattern lowering: AstPat → HirPat.
//!
//! Allocates local variable slots for pattern bindings and resolves
//! enum/struct pattern names to entities where possible.

use kestrel_ast::ast_body::*;
use kestrel_hir::body::*;
use kestrel_name_res::{ResolveTypePath, ResolveValuePath, TypeResolution, ValueResolution};
use kestrel_span::Span;

use crate::ctx::{LowerCtx, name_from_ast};

impl LowerCtx<'_> {
    /// Lower an AST pattern to an HIR pattern.
    /// Callers that don't inherit mutability from an outer `var` should use this.
    pub fn lower_pat(&mut self, body: &AstBody, id: PatId) -> HirPatId {
        self.lower_pat_inner(body, id, false)
    }

    /// Lower an AST pattern, forcing all bindings mutable.
    /// Used by `var <pattern> = …` destructuring so the outer `var` propagates
    /// into every binding the pattern introduces.
    pub fn lower_pat_forcing_mut(
        &mut self,
        body: &AstBody,
        id: PatId,
        force_mut: bool,
    ) -> HirPatId {
        self.lower_pat_inner(body, id, force_mut)
    }

    fn lower_pat_inner(&mut self, body: &AstBody, id: PatId, force_mut: bool) -> HirPatId {
        let pat = &body.pats[id];
        match pat {
            AstPat::Wildcard { span } => self.alloc_pat(HirPat::Wildcard { span: span.clone() }),

            AstPat::Binding { is_mut, name, span } => {
                let local = self.define_local(name, *is_mut || force_mut, span.clone());
                self.alloc_pat(HirPat::Binding {
                    local,
                    span: span.clone(),
                })
            },

            AstPat::Tuple {
                prefix,
                has_rest,
                multiple_rests,
                suffix,
                span,
            } => {
                if *multiple_rests {
                    self.ctx.accumulate(
                        kestrel_reporting::Diagnostic::error()
                            .with_message(
                                "only one rest pattern (`..`) is allowed per tuple pattern",
                            )
                            .with_labels(vec![
                                kestrel_reporting::Label::primary(span.file_id, span.range())
                                    .with_message("multiple rest patterns found"),
                            ]),
                    );
                }
                let lowered_prefix: Vec<HirPatId> = prefix
                    .iter()
                    .map(|&id| self.lower_pat_inner(body, id, force_mut))
                    .collect();
                let lowered_suffix: Vec<HirPatId> = suffix
                    .iter()
                    .map(|&id| self.lower_pat_inner(body, id, force_mut))
                    .collect();
                self.alloc_pat(HirPat::Tuple {
                    prefix: lowered_prefix,
                    has_rest: *has_rest,
                    suffix: lowered_suffix,
                    span: span.clone(),
                })
            },

            AstPat::Literal { kind, span } => {
                let value = lower_lit_pat(kind, span);
                self.alloc_pat(HirPat::Literal {
                    value,
                    span: span.clone(),
                })
            },

            AstPat::Range {
                start,
                end,
                inclusive,
                span,
            } => {
                let hir_start = start.as_ref().map(|k| lower_lit_pat(k, span));
                let hir_end = end.as_ref().map(|k| lower_lit_pat(k, span));

                // Validate: start must be <= end (inclusive) or < end (exclusive)
                if let (Some(s), Some(e)) = (&hir_start, &hir_end) {
                    let invalid = match (s, e) {
                        (HirLiteral::Integer(s), HirLiteral::Integer(e)) => {
                            if *inclusive {
                                s > e
                            } else {
                                s >= e
                            }
                        },
                        (HirLiteral::Char(s), HirLiteral::Char(e)) => {
                            if *inclusive {
                                s > e
                            } else {
                                s >= e
                            }
                        },
                        _ => false,
                    };
                    if invalid {
                        self.ctx.accumulate(
                            kestrel_reporting::Diagnostic::error()
                                .with_message(
                                    "invalid range bounds: start must be less than or equal to end",
                                )
                                .with_labels(vec![
                                    kestrel_reporting::Label::primary(span.file_id, span.range())
                                        .with_message("range bounds are reversed"),
                                ]),
                        );
                    }
                }

                self.alloc_pat(HirPat::Range {
                    start: hir_start,
                    end: hir_end,
                    inclusive: *inclusive,
                    span: span.clone(),
                })
            },

            AstPat::Enum {
                case_name,
                args,
                span,
            } => self.lower_enum_pat(body, case_name, args, span, force_mut),

            AstPat::Struct {
                name,
                fields,
                has_rest,
                span,
            } => self.lower_struct_pat(body, name, fields, *has_rest, span, force_mut),

            AstPat::Array {
                prefix,
                rest,
                suffix,
                span,
            } => {
                let lowered_prefix: Vec<HirPatId> = prefix
                    .iter()
                    .map(|&id| self.lower_pat_inner(body, id, force_mut))
                    .collect();
                // Map Option<Option<String>> → Option<Option<LocalId>>:
                // - None → None (no rest)
                // - Some(None) → Some(None) (bare `..`)
                // - Some(Some(name)) → Some(Some(local)) (named `..name`, inherits outer `var`)
                let hir_rest = rest.as_ref().map(|inner| {
                    inner
                        .as_ref()
                        .map(|name| self.define_local(name, force_mut, span.clone()))
                });
                let lowered_suffix: Vec<HirPatId> = suffix
                    .iter()
                    .map(|&id| self.lower_pat_inner(body, id, force_mut))
                    .collect();
                self.alloc_pat(HirPat::Array {
                    prefix: lowered_prefix,
                    rest: hir_rest,
                    suffix: lowered_suffix,
                    span: span.clone(),
                })
            },

            AstPat::At {
                is_mut,
                name,
                subpattern,
                span,
            } => {
                // Check for nested @ patterns
                if matches!(&body.pats[*subpattern], AstPat::At { .. }) {
                    self.ctx.accumulate(
                        kestrel_reporting::Diagnostic::error()
                            .with_message("nested @ patterns are not allowed")
                            .with_labels(vec![
                                kestrel_reporting::Label::primary(span.file_id, span.range())
                                    .with_message(
                                        "use a single @ pattern with the outermost binding",
                                    ),
                            ]),
                    );
                    // Still define the outer binding so arm-body references
                    // resolve, but replace the subpattern with Error so the
                    // exhaustiveness pass skips this arm instead of seeing
                    // an irrefutable @-over-wildcard.
                    let local = self.define_local(name, *is_mut || force_mut, span.clone());
                    let err_sub = self.alloc_pat(HirPat::Error { span: span.clone() });
                    return self.alloc_pat(HirPat::At {
                        binding: local,
                        subpattern: err_sub,
                        span: span.clone(),
                    });
                }

                let local = self.define_local(name, *is_mut || force_mut, span.clone());
                let lowered_sub = self.lower_pat_inner(body, *subpattern, force_mut);
                self.alloc_pat(HirPat::At {
                    binding: local,
                    subpattern: lowered_sub,
                    span: span.clone(),
                })
            },

            AstPat::Or { alternatives, span } => {
                let lowered: Vec<HirPatId> = alternatives
                    .iter()
                    .map(|&id| self.lower_pat_inner(body, id, force_mut))
                    .collect();
                self.alloc_pat(HirPat::Or {
                    alternatives: lowered,
                    span: span.clone(),
                })
            },

            AstPat::Rest { span } => {
                // Rest should be absorbed by parent — standalone is an error
                self.alloc_pat(HirPat::Error { span: span.clone() })
            },

            AstPat::Error { span } => self.alloc_pat(HirPat::Error { span: span.clone() }),
        }
    }

    /// Lower an enum pattern. Try to resolve the case name to an entity.
    fn lower_enum_pat(
        &mut self,
        body: &AstBody,
        case_name: &str,
        args: &[EnumPatArg],
        span: &Span,
        force_mut: bool,
    ) -> HirPatId {
        let lowered_args: Vec<HirPatArg> = args
            .iter()
            .map(|arg| HirPatArg {
                label: arg.label.clone(),
                pattern: self.lower_pat_inner(body, arg.pattern, force_mut),
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
                        name: name_from_ast(case_name.to_string()),
                        args: lowered_args,
                        span: span.clone(),
                    })
                }
            },
            _ => {
                // Not found or ambiguous — leave as implicit for type inference
                self.alloc_pat(HirPat::ImplicitVariant {
                    name: name_from_ast(case_name.to_string()),
                    args: lowered_args,
                    span: span.clone(),
                })
            },
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
        force_mut: bool,
    ) -> HirPatId {
        let lowered_fields: Vec<HirStructPatField> = fields
            .iter()
            .map(|f| {
                // Shorthand fields (Point { x }) have pattern: None — create a binding.
                // Shorthand bindings inherit outer `var` via force_mut.
                let pattern = if let Some(id) = f.pattern {
                    Some(self.lower_pat_inner(body, id, force_mut))
                } else {
                    let local = self.define_local(&f.field_name, force_mut, span.clone());
                    Some(self.alloc_pat(HirPat::Binding {
                        local,
                        span: span.clone(),
                    }))
                };
                HirStructPatField {
                    field_name: name_from_ast(f.field_name.clone()),
                    pattern,
                }
            })
            .collect();

        // Try to resolve struct name
        let result = self.ctx.query(ResolveTypePath {
            segments: vec![name.to_string()],
            context: self.owner,
            root: self.root,
        });

        match result {
            TypeResolution::Found(entity) => {
                // Validate pattern fields against struct's actual fields
                use kestrel_ast_builder::{Name, NodeKind};
                let struct_field_names: Vec<String> = self
                    .ctx
                    .children_of(entity)
                    .iter()
                    .filter(|&&c| self.ctx.get::<NodeKind>(c) == Some(&NodeKind::Field))
                    .filter_map(|&c| self.ctx.get::<Name>(c).map(|n| n.0.clone()))
                    .collect();

                // Check for unknown fields. Skip pattern fields whose name is
                // `Missing` — the parser already reported the gap; "no field
                // ``" would just be cascade noise.
                let mut has_unknown = false;
                for field in &lowered_fields {
                    let Some(field_name) = field.field_name.as_str() else {
                        continue;
                    };
                    if !struct_field_names.iter().any(|n| n == field_name) {
                        has_unknown = true;
                        self.ctx.accumulate(
                            kestrel_reporting::Diagnostic::error()
                                .with_message(format!(
                                    "struct `{}` has no field `{}`",
                                    name, field_name
                                ))
                                .with_labels(vec![
                                    kestrel_reporting::Label::primary(span.file_id, span.range())
                                        .with_message(format!("unknown field `{}`", field_name)),
                                ]),
                        );
                    }
                }

                // Check for missing fields (unless has_rest `..` or unknown fields present)
                if !has_rest && !has_unknown {
                    let matched: std::collections::HashSet<&str> = lowered_fields
                        .iter()
                        .filter_map(|f| f.field_name.as_str())
                        .collect();
                    let missing: Vec<&str> = struct_field_names
                        .iter()
                        .filter(|f| !matched.contains(f.as_str()))
                        .map(|f| f.as_str())
                        .collect();
                    if !missing.is_empty() {
                        self.ctx.accumulate(
                            kestrel_reporting::Diagnostic::error()
                                .with_message(format!(
                                    "pattern does not cover field{} `{}`",
                                    if missing.len() > 1 { "s" } else { "" },
                                    missing.join("`, `"),
                                ))
                                .with_labels(vec![
                                    kestrel_reporting::Label::primary(span.file_id, span.range())
                                        .with_message("use `..` to ignore remaining fields"),
                                ]),
                        );
                    }
                }

                self.alloc_pat(HirPat::Struct {
                    entity,
                    fields: lowered_fields,
                    has_rest,
                    span: span.clone(),
                })
            },
            _ => self.alloc_pat(HirPat::Error { span: span.clone() }),
        }
    }

    /// Lower a ParamPattern (from parameter destructuring) to an HirPat.
    /// `force_mut` makes all bindings mutable (for `mutating` access mode).
    pub fn lower_param_pattern(
        &mut self,
        pattern: &kestrel_ast_builder::ParamPattern,
        span: &Span,
        force_mut: bool,
    ) -> HirPatId {
        match pattern {
            kestrel_ast_builder::ParamPattern::Wildcard => {
                self.alloc_pat(HirPat::Wildcard { span: span.clone() })
            },

            kestrel_ast_builder::ParamPattern::Binding { name, is_mut } => {
                let local = self.define_local(name, *is_mut || force_mut, span.clone());
                self.alloc_pat(HirPat::Binding {
                    local,
                    span: span.clone(),
                })
            },

            kestrel_ast_builder::ParamPattern::Tuple { elements } => {
                let lowered: Vec<HirPatId> = elements
                    .iter()
                    .map(|elem| self.lower_param_pattern(elem, span, force_mut))
                    .collect();
                self.alloc_pat(HirPat::Tuple {
                    prefix: lowered,
                    has_rest: false,
                    suffix: vec![],
                    span: span.clone(),
                })
            },

            kestrel_ast_builder::ParamPattern::Struct {
                type_name,
                fields,
                has_rest,
            } => {
                let lowered_fields: Vec<HirStructPatField> = fields
                    .iter()
                    .map(|f| HirStructPatField {
                        field_name: name_from_ast(f.field_name.clone()),
                        pattern: Some(self.lower_param_pattern(&f.pattern, span, force_mut)),
                    })
                    .collect();

                // Resolve struct name
                let result = self.ctx.query(ResolveTypePath {
                    segments: vec![type_name.to_string()],
                    context: self.owner,
                    root: self.root,
                });

                match result {
                    TypeResolution::Found(entity) => self.alloc_pat(HirPat::Struct {
                        entity,
                        fields: lowered_fields,
                        has_rest: *has_rest,
                        span: span.clone(),
                    }),
                    _ => self.alloc_pat(HirPat::Error { span: span.clone() }),
                }
            },
        }
    }
}

/// Convert a literal pattern kind to an HIR literal. `span` is the span of
/// the literal's source text, used to compute escape-error sub-spans for
/// string patterns.
fn lower_lit_pat(kind: &LitPatKind, span: &Span) -> HirLiteral {
    match kind {
        LitPatKind::Integer(s) => HirLiteral::Integer(parse_int(s)),
        LitPatKind::Float(s) => HirLiteral::Float(parse_float(s)),
        LitPatKind::String(s) => {
            let (value, escape_errors) =
                crate::literal::decode_string_literal_token(s, span.file_id, span.start);
            HirLiteral::String {
                value,
                escape_errors,
            }
        },
        LitPatKind::Bool(b) => HirLiteral::Bool(*b),
        LitPatKind::Char(s) => HirLiteral::Char(parse_char(s)),
    }
}

/// Parse an integer literal string to i64.
///
/// For values above `i64::MAX` but within `u64::MAX`, parses as `u64` and
/// reinterprets the bit pattern as `i64` so unsigned literals like
/// `UInt64.maxValue = 18446744073709551615` round-trip correctly.
pub(crate) fn parse_int(s: &str) -> i64 {
    let s = s.replace('_', "");
    let (body, radix) = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (hex, 16)
    } else if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        (oct, 8)
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        (bin, 2)
    } else {
        (s.as_str(), 10)
    };
    i64::from_str_radix(body, radix)
        .or_else(|_| u64::from_str_radix(body, radix).map(|u| u as i64))
        .unwrap_or(0)
}

/// Parse a float literal string to f64.
pub(crate) fn parse_float(s: &str) -> f64 {
    s.replace('_', "").parse().unwrap_or(0.0)
}

/// Parse a char literal string to a unicode scalar value (no validation).
/// Used for pattern literals where we don't have diagnostic context.
pub(crate) fn parse_char(s: &str) -> u32 {
    // Strip exactly one quote from each end (trim_matches strips ALL matching chars,
    // which breaks '\'' by also stripping the escaped quote content)
    let inner = s.strip_prefix('\'').unwrap_or(s);
    let inner = inner.strip_suffix('\'').unwrap_or(inner);
    let codepoints = unescape_char_content(inner, &Span::synthetic(0), None);
    codepoints.first().copied().unwrap_or(0)
}

/// Parse and validate a char literal, emitting diagnostics for invalid content.
/// Used during HIR lowering where diagnostic context is available.
pub(crate) fn parse_char_validated(
    s: &str,
    span: &Span,
    ctx: &kestrel_hecs::QueryContext<'_>,
) -> u32 {
    // Strip exactly one quote from each end (trim_matches strips ALL matching chars,
    // which breaks '\'' by also stripping the escaped quote content)
    let inner = s.strip_prefix('\'').unwrap_or(s);
    let inner = inner.strip_suffix('\'').unwrap_or(inner);

    // Empty char literal
    if inner.is_empty() {
        ctx.accumulate(
            kestrel_reporting::Diagnostic::error()
                .with_message("empty character literal")
                .with_labels(vec![
                    kestrel_reporting::Label::primary(span.file_id, span.range())
                        .with_message("character literal must contain exactly one codepoint"),
                ]),
        );
        return 0;
    }

    let codepoints = unescape_char_content(inner, span, Some(ctx));

    if codepoints.is_empty() {
        // Escape processing consumed everything but produced nothing (shouldn't happen)
        return 0;
    }

    if codepoints.len() > 1 {
        ctx.accumulate(
            kestrel_reporting::Diagnostic::error()
                .with_message("character literal may only contain one codepoint")
                .with_labels(vec![
                    kestrel_reporting::Label::primary(span.file_id, span.range())
                        .with_message(format!("found {} codepoints", codepoints.len())),
                ]),
        );
    }

    codepoints[0]
}

/// Process escape sequences in char literal content, returning codepoints.
/// If `ctx` is provided, emits diagnostics for invalid escapes.
fn unescape_char_content(
    s: &str,
    span: &Span,
    ctx: Option<&kestrel_hecs::QueryContext<'_>>,
) -> Vec<u32> {
    let mut result = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n' as u32),
                Some('r') => result.push('\r' as u32),
                Some('t') => result.push('\t' as u32),
                Some('\\') => result.push('\\' as u32),
                Some('\'') => result.push('\'' as u32),
                Some('"') => result.push('"' as u32),
                Some('0') => result.push(0),
                Some('x') => {
                    // Hex ASCII escape: \xNN (exactly 2 hex digits, value ≤ 0x7F)
                    let d1 = chars.next();
                    let d2 = chars.next();
                    match (d1, d2) {
                        (Some(h1), Some(h2))
                            if h1.is_ascii_hexdigit() && h2.is_ascii_hexdigit() =>
                        {
                            let hex_str: String = [h1, h2].iter().collect();
                            let value = u32::from_str_radix(&hex_str, 16).unwrap_or(0);
                            if value > 0x7F
                                && let Some(ctx) = ctx
                            {
                                ctx.accumulate(
                                    kestrel_reporting::Diagnostic::error()
                                        .with_message(format!(
                                            "ASCII escape \\x{:02X} out of range",
                                            value
                                        ))
                                        .with_labels(vec![
                                            kestrel_reporting::Label::primary(
                                                span.file_id,
                                                span.range(),
                                            )
                                            .with_message("must be in range \\x00-\\x7F"),
                                        ]),
                                );
                            }
                            result.push(value);
                        },
                        _ => {
                            // Incomplete hex escape
                            if let Some(ctx) = ctx {
                                ctx.accumulate(
                                    kestrel_reporting::Diagnostic::error()
                                        .with_message("invalid escape sequence")
                                        .with_labels(vec![
                                            kestrel_reporting::Label::primary(
                                                span.file_id,
                                                span.range(),
                                            )
                                            .with_message("incomplete hex escape (expected \\xNN)"),
                                        ]),
                                );
                            }
                            result.push(0);
                        },
                    }
                },
                Some('u') => {
                    // Unicode escape: \u{NNNN} (1-6 hex digits)
                    if chars.next() != Some('{') {
                        if let Some(ctx) = ctx {
                            ctx.accumulate(
                                kestrel_reporting::Diagnostic::error()
                                    .with_message("invalid Unicode escape")
                                    .with_labels(vec![
                                        kestrel_reporting::Label::primary(
                                            span.file_id,
                                            span.range(),
                                        )
                                        .with_message("expected '{{' after \\u"),
                                    ]),
                            );
                        }
                        result.push(0);
                        continue;
                    }
                    let mut hex = String::new();
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        hex.push(c);
                    }
                    match u32::from_str_radix(&hex, 16) {
                        Ok(value) if value > 0x10FFFF => {
                            if let Some(ctx) = ctx {
                                ctx.accumulate(
                                    kestrel_reporting::Diagnostic::error()
                                        .with_message("invalid Unicode escape")
                                        .with_labels(vec![
                                            kestrel_reporting::Label::primary(
                                                span.file_id,
                                                span.range(),
                                            )
                                            .with_message(format!(
                                                "\\u{{{}}} is out of range (max 10FFFF)",
                                                hex
                                            )),
                                        ]),
                                );
                            }
                            result.push(0);
                        },
                        Ok(value) if (0xD800..=0xDFFF).contains(&value) => {
                            if let Some(ctx) = ctx {
                                ctx.accumulate(
                                    kestrel_reporting::Diagnostic::error()
                                        .with_message("invalid Unicode escape")
                                        .with_labels(vec![
                                            kestrel_reporting::Label::primary(
                                                span.file_id,
                                                span.range(),
                                            )
                                            .with_message(format!(
                                                "\\u{{{}}} is a surrogate codepoint",
                                                hex
                                            )),
                                        ]),
                                );
                            }
                            result.push(0);
                        },
                        Ok(value) => result.push(value),
                        Err(_) => {
                            if let Some(ctx) = ctx {
                                ctx.accumulate(
                                    kestrel_reporting::Diagnostic::error()
                                        .with_message("invalid Unicode escape")
                                        .with_labels(vec![
                                            kestrel_reporting::Label::primary(
                                                span.file_id,
                                                span.range(),
                                            )
                                            .with_message("invalid hex digits"),
                                        ]),
                                );
                            }
                            result.push(0);
                        },
                    }
                },
                Some(esc) => {
                    // Unknown escape sequence
                    if let Some(ctx) = ctx {
                        ctx.accumulate(
                            kestrel_reporting::Diagnostic::error()
                                .with_message(format!("invalid escape sequence '\\{}'", esc))
                                .with_labels(vec![
                                    kestrel_reporting::Label::primary(span.file_id, span.range())
                                        .with_message("unknown escape"),
                                ]),
                        );
                    }
                    result.push(esc as u32);
                },
                None => {
                    // Backslash at end of literal
                    result.push('\\' as u32);
                },
            }
        } else {
            result.push(c as u32);
        }
    }

    result
}
