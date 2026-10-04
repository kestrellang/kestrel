//! Pattern lowering: CST patterns → HirPat.
//!
//! Allocates local variable slots for pattern bindings and resolves
//! enum/struct pattern names to entities where possible.

use kestrel_ast::escape::{Escaped, decode_escape};
use kestrel_hir::body::*;
use kestrel_name_res::{ResolveTypePath, ResolveValuePath, TypeResolution, ValueResolution};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ctx::{LowerCtx, hir_name};
use crate::syntax::{
    PatSrc, enum_arg_label, first_pat, has_token, is_pat_like, token, tuple_pattern_elements,
};

/// A literal in a pattern, as written.
enum LitPat {
    Integer(String),
    Float(String),
    String(String),
    Bool(bool),
    Char(String),
}

/// The literal a pattern token spells.
fn lit_pat(token: &SyntaxToken) -> Option<LitPat> {
    let text = token.text().to_string();
    Some(match token.kind() {
        SyntaxKind::Integer => LitPat::Integer(text),
        SyntaxKind::Float => LitPat::Float(text),
        SyntaxKind::String => LitPat::String(text),
        SyntaxKind::Boolean => LitPat::Bool(text == "true"),
        SyntaxKind::Char => LitPat::Char(text),
        _ => return None,
    })
}

/// One argument of an enum pattern.
struct EnumArg {
    label: Option<String>,
    pat: PatSrc,
}

/// One field of a struct pattern; `pat` is `None` for the `{ x }` shorthand.
struct StructField {
    name: Option<SyntaxToken>,
    pat: Option<PatSrc>,
}

impl LowerCtx<'_> {
    /// Lower a pattern.
    /// Callers that don't inherit mutability from an outer `var` should use this.
    pub(crate) fn lower_pat(&mut self, pat: &PatSrc) -> HirPatId {
        self.lower_pat_inner(pat, false)
    }

    /// Lower a pattern, forcing all bindings mutable.
    /// Used by `var <pattern> = …` destructuring so the outer `var` propagates
    /// into every binding the pattern introduces.
    pub(crate) fn lower_pat_forcing_mut(&mut self, pat: &PatSrc, force_mut: bool) -> HirPatId {
        self.lower_pat_inner(pat, force_mut)
    }

    fn lower_pat_inner(&mut self, pat: &PatSrc, force_mut: bool) -> HirPatId {
        match pat.clone().resolve(self.file_id) {
            PatSrc::Error(span) => self.alloc_pat(HirPat::Error { span }),
            PatSrc::ArgBinding(arg) => {
                let span = self.span(&arg);
                match token(&arg, SyntaxKind::Identifier) {
                    Some(name) => {
                        let local = self.define_named_local(&arg, &name, force_mut, span.clone());
                        self.alloc_pat(HirPat::Binding {
                            local,
                            by_ref: None,
                            span,
                        })
                    },
                    None => self.alloc_pat(HirPat::Error { span }),
                }
            },
            PatSrc::Node(node) => {
                let id = self.lower_pat_node(&node, force_mut);
                self.source_map.record_pat(&node, id);
                id
            },
        }
    }

    fn lower_pat_node(&mut self, node: &SyntaxNode, force_mut: bool) -> HirPatId {
        let span = self.span(node);
        match node.kind() {
            SyntaxKind::WildcardPattern => self.alloc_pat(HirPat::Wildcard { span }),

            SyntaxKind::BindingPattern | SyntaxKind::RefBindingPattern => {
                let by_ref = (node.kind() == SyntaxKind::RefBindingPattern)
                    .then(|| has_token(node, SyntaxKind::Mutating));
                let is_mut = by_ref.is_none() && has_token(node, SyntaxKind::Var);
                // `&`/`&mutating` binder patterns (stage 1.5 item 2) are
                // match-arm constructs: the place-mode lowering needs a
                // pinnable scrutinee place. Everywhere else (let/for
                // destructures, if/while-let conditions, params) they
                // reject (E211) and degrade to a plain binding so
                // downstream diagnostics stay useful.
                let allowed = self.ref_patterns_allowed;
                if by_ref.is_some() && !allowed {
                    self.ctx.accumulate(
                        kestrel_reporting::Diagnostic::error()
                            .with_code("E211")
                            .with_message("`&` pattern bindings are not supported in this position")
                            .with_labels(vec![
                                kestrel_reporting::Label::primary(span.file_id, span.range())
                                    .with_message("`&` binder pattern"),
                            ])
                            .with_notes(vec![
                                "`&` binders are supported in `match` arm patterns".to_string(),
                            ]),
                    );
                }
                // A binder whose name the parser could not find binds nothing.
                let Some(name) = token(node, SyntaxKind::Identifier) else {
                    return self.alloc_pat(HirPat::Error { span });
                };
                let local = self.define_named_local(node, &name, is_mut || force_mut, span.clone());
                self.alloc_pat(HirPat::Binding {
                    local,
                    by_ref: if allowed { by_ref } else { None },
                    span,
                })
            },

            SyntaxKind::TuplePattern => {
                let elements = tuple_pattern_elements(node, self.file_id);
                let rests: Vec<usize> = elements
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.is_rest)
                    .map(|(i, _)| i)
                    .collect();
                if rests.len() > 1 {
                    self.ctx.accumulate(
                        kestrel_reporting::Diagnostic::error()
                            .with_code("E317")
                            .with_message(
                                "only one rest pattern (`..`) is allowed per tuple pattern",
                            )
                            .with_labels(vec![
                                kestrel_reporting::Label::primary(span.file_id, span.range())
                                    .with_message("multiple rest patterns found"),
                            ]),
                    );
                }
                // Split at the first rest; anything after it (a second rest
                // included, which lowers to an error) is the suffix.
                let (prefix, suffix) = match rests.first() {
                    Some(&r) => (&elements[..r], &elements[r + 1..]),
                    None => (&elements[..], &elements[elements.len()..]),
                };
                let lowered_prefix: Vec<HirPatId> = prefix
                    .iter()
                    .map(|e| self.lower_pat_inner(&e.pat, force_mut))
                    .collect();
                let lowered_suffix: Vec<HirPatId> = suffix
                    .iter()
                    .map(|e| self.lower_pat_inner(&e.pat, force_mut))
                    .collect();
                self.alloc_pat(HirPat::Tuple {
                    prefix: lowered_prefix,
                    has_rest: !rests.is_empty(),
                    suffix: lowered_suffix,
                    span,
                })
            },

            SyntaxKind::LiteralPattern => {
                let kind = crate::syntax::first_token(node)
                    .map(|t| lit_pat(&t).unwrap_or(LitPat::Integer(t.text().to_string())))
                    .unwrap_or(LitPat::Integer("0".into()));
                let value = lower_lit_pat(&kind, &span);
                self.alloc_pat(HirPat::Literal { value, span })
            },

            SyntaxKind::RangePattern => self.lower_range_pat(node, span),

            SyntaxKind::EnumPattern => {
                let case_name = token(node, SyntaxKind::Identifier).map(|t| t.text().to_string());
                let args: Vec<EnumArg> = node
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::EnumPatternArg)
                    .map(|arg| EnumArg {
                        label: enum_arg_label(&arg),
                        pat: first_pat(&arg).map_or(PatSrc::ArgBinding(arg), PatSrc::Node),
                    })
                    .collect();
                self.lower_enum_pat(case_name, &args, &span, force_mut)
            },

            // `null` is `Optional.None`; type checking enforces the scrutinee
            // is an optional through ordinary case resolution.
            SyntaxKind::NullPattern => {
                self.lower_enum_pat(Some("None".to_string()), &[], &span, force_mut)
            },

            // `some PAT` is `.Some(PAT)`; nested sugar (`some some x`,
            // `some null`) flows through.
            SyntaxKind::SomePattern => {
                let args = [EnumArg {
                    label: None,
                    pat: PatSrc::or_error(first_pat(node), &span),
                }];
                self.lower_enum_pat(Some("Some".to_string()), &args, &span, force_mut)
            },

            SyntaxKind::StructPattern => {
                let name = token(node, SyntaxKind::Identifier).map(|t| t.text().to_string());
                let fields: Vec<StructField> = node
                    .children()
                    .filter(|c| c.kind() == SyntaxKind::StructPatternField)
                    .map(|field| StructField {
                        name: token(&field, SyntaxKind::Identifier),
                        pat: first_pat(&field).map(PatSrc::Node),
                    })
                    .collect();
                let has_rest = node
                    .children()
                    .any(|c| c.kind() == SyntaxKind::StructPatternRest);
                self.lower_struct_pat(name, &fields, has_rest, &span, force_mut)
            },

            SyntaxKind::ArrayPattern => {
                let mut prefix = Vec::new();
                let mut rest: Option<SyntaxNode> = None;
                let mut suffix = Vec::new();
                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::ArrayPatternElement => {
                            let child_span = self.span(&child);
                            let pat = PatSrc::or_error(first_pat(&child), &child_span);
                            if rest.is_some() {
                                suffix.push(pat);
                            } else {
                                prefix.push(pat);
                            }
                        },
                        SyntaxKind::ArrayPatternRest => rest = Some(child),
                        _ => {},
                    }
                }
                let lowered_prefix: Vec<HirPatId> = prefix
                    .iter()
                    .map(|p| self.lower_pat_inner(p, force_mut))
                    .collect();
                // No rest → None; bare `..` → Some(None); named `..name` →
                // Some(Some(local)), inheriting an outer `var`.
                let hir_rest = rest.map(|rest| {
                    token(&rest, SyntaxKind::Identifier)
                        .map(|name| self.define_named_local(&rest, &name, force_mut, span.clone()))
                });
                let lowered_suffix: Vec<HirPatId> = suffix
                    .iter()
                    .map(|p| self.lower_pat_inner(p, force_mut))
                    .collect();
                self.alloc_pat(HirPat::Array {
                    prefix: lowered_prefix,
                    rest: hir_rest,
                    suffix: lowered_suffix,
                    span,
                })
            },

            SyntaxKind::AtPattern => {
                let is_mut = has_token(node, SyntaxKind::Var) || force_mut;
                let subpattern = PatSrc::or_error(first_pat(node), &span);
                // `@ pat` whose binder name is missing (already reported by
                // the parser) is just its subpattern.
                let Some(name) = token(node, SyntaxKind::Identifier) else {
                    return self.lower_pat_inner(&subpattern, force_mut);
                };
                // Check for nested @ patterns
                let nested =
                    subpattern.clone().resolve(self.file_id).kind() == Some(SyntaxKind::AtPattern);
                if nested {
                    self.ctx.accumulate(
                        kestrel_reporting::Diagnostic::error()
                            .with_code("E319")
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
                    let local = self.define_named_local(node, &name, is_mut, span.clone());
                    let err_sub = self.alloc_pat(HirPat::Error { span: span.clone() });
                    return self.alloc_pat(HirPat::At {
                        binding: local,
                        subpattern: err_sub,
                        span,
                    });
                }

                let local = self.define_named_local(node, &name, is_mut, span.clone());
                let lowered_sub = self.lower_pat_inner(&subpattern, force_mut);
                self.alloc_pat(HirPat::At {
                    binding: local,
                    subpattern: lowered_sub,
                    span,
                })
            },

            SyntaxKind::OrPattern => {
                let alternatives: Vec<PatSrc> = node
                    .children()
                    .filter(|c| is_pat_like(c.kind()))
                    .map(PatSrc::Node)
                    .collect();
                // Lower the first alternative normally, then make every later
                // alternative reuse the locals it created (per binding name) so
                // all alternatives — and the arm body — share one local per
                // name. Without this, each `.A(x) or .B(x)` alternative gets a
                // distinct `x`; the body reads the last-defined one while each
                // leaf binds its own → an undefined-local OSSA ICE (#187).
                let mut lowered: Vec<HirPatId> = Vec::with_capacity(alternatives.len());
                let mut iter = alternatives.iter();
                if let Some(first) = iter.next() {
                    let before = self.current_scope_bindings();
                    lowered.push(self.lower_pat_inner(first, force_mut));
                    let after = self.current_scope_bindings();
                    // Names the first alternative (re)bound → reuse for the rest.
                    let reuse: std::collections::HashMap<String, _> = after
                        .into_iter()
                        .filter(|(name, local)| before.get(name) != Some(local))
                        .collect();
                    let prev = self.set_or_reuse(Some(reuse));
                    for alt in iter {
                        lowered.push(self.lower_pat_inner(alt, force_mut));
                    }
                    self.set_or_reuse(prev);
                }
                self.alloc_pat(HirPat::Or {
                    alternatives: lowered,
                    span,
                })
            },

            // A rest outside a tuple/array (absorbed there) and recovery
            // nodes are errors.
            _ => self.alloc_pat(HirPat::Error { span }),
        }
    }

    /// `lo..hi`, `lo..=hi`, `..<hi`, `lo..`.
    fn lower_range_pat(&mut self, node: &SyntaxNode, span: Span) -> HirPatId {
        let inclusive = has_token(node, SyntaxKind::DotDotEquals);
        // Bounds are the literal tokens before/after the range operator.
        let mut before_op = true;
        let mut start = None;
        let mut end = None;
        for token in node
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
        {
            match token.kind() {
                SyntaxKind::DotDotEquals | SyntaxKind::DotDotLess | SyntaxKind::DotDot => {
                    before_op = false;
                },
                _ => {
                    let Some(lit) = lit_pat(&token) else {
                        continue;
                    };
                    if before_op {
                        start = Some(lit);
                    } else {
                        end = Some(lit);
                    }
                },
            }
        }
        let hir_start = start.as_ref().map(|k| lower_lit_pat(k, &span));
        let hir_end = end.as_ref().map(|k| lower_lit_pat(k, &span));

        // Validate: start must be <= end (inclusive) or < end (exclusive)
        if let (Some(s), Some(e)) = (&hir_start, &hir_end) {
            let invalid = match (s, e) {
                (HirLiteral::Integer(s), HirLiteral::Integer(e)) => {
                    if inclusive {
                        s > e
                    } else {
                        s >= e
                    }
                },
                (HirLiteral::Char { value: s, .. }, HirLiteral::Char { value: e, .. }) => {
                    if inclusive {
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
                        .with_code("E318")
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
            inclusive,
            span,
        })
    }

    /// Lower an enum pattern. Try to resolve the case name to an entity.
    fn lower_enum_pat(
        &mut self,
        case_name: Option<String>,
        args: &[EnumArg],
        span: &Span,
        force_mut: bool,
    ) -> HirPatId {
        let lowered_args: Vec<HirPatArg> = args
            .iter()
            .map(|arg| HirPatArg {
                label: arg.label.clone(),
                pattern: self.lower_pat_inner(&arg.pat, force_mut),
            })
            .collect();

        // A case name the parser could not find stays implicit (and missing).
        let Some(case_name) = case_name else {
            return self.alloc_pat(HirPat::ImplicitVariant {
                name: HirName::Missing,
                args: lowered_args,
                span: span.clone(),
            });
        };

        // Try to resolve as a qualified enum case (e.g. "MyEnum.caseA" if multi-segment,
        // or just "CaseName" if it's a known enum case in scope)
        let result = self.ctx.query(ResolveValuePath {
            segments: vec![case_name.clone()],
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
                        name: HirName::Name(case_name.clone()),
                        args: lowered_args,
                        span: span.clone(),
                    })
                }
            },
            _ => {
                // Not found or ambiguous — leave as implicit for type inference
                self.alloc_pat(HirPat::ImplicitVariant {
                    name: HirName::Name(case_name.clone()),
                    args: lowered_args,
                    span: span.clone(),
                })
            },
        }
    }

    /// Lower a struct pattern. Resolve the struct name to an entity.
    fn lower_struct_pat(
        &mut self,
        name: Option<String>,
        fields: &[StructField],
        has_rest: bool,
        span: &Span,
        force_mut: bool,
    ) -> HirPatId {
        let lowered_fields: Vec<HirStructPatField> = fields
            .iter()
            .map(|f| {
                // Shorthand fields (Point { x }) have no pattern — they bind
                // the field's name. Shorthand bindings inherit outer `var` via
                // force_mut.
                let pattern = match (&f.pat, &f.name) {
                    (Some(pat), _) => self.lower_pat_inner(pat, force_mut),
                    // Not recorded as a named declaration: the token is the
                    // field's name too, so renaming the local through it
                    // would rename the field.
                    (None, Some(field_name)) => {
                        let local = self.define_local(field_name.text(), force_mut, span.clone());
                        self.alloc_pat(HirPat::Binding {
                            local,
                            by_ref: None,
                            span: span.clone(),
                        })
                    },
                    (None, None) => self.alloc_pat(HirPat::Error { span: span.clone() }),
                };
                HirStructPatField {
                    field_name: hir_name(f.name.as_ref().map(|t| t.text().to_string())),
                    pattern: Some(pattern),
                }
            })
            .collect();

        // A struct name the parser could not find resolves to nothing.
        let Some(name) = name else {
            return self.alloc_pat(HirPat::Error { span: span.clone() });
        };
        let result = self.ctx.query(ResolveTypePath {
            segments: vec![name.clone()],
            context: self.owner,
            root: self.root,
        });

        match result {
            TypeResolution::Found(entity) => {
                // Validate pattern fields against struct's actual fields
                // Only stored instance fields are bindable: a pattern destructures
                // storage, and this set must match the layout roster used to
                // assign sub-pattern indices.
                use kestrel_ast_builder::{FieldClass, Name};
                let struct_field_names: Vec<String> = self
                    .ctx
                    .children_of(entity)
                    .iter()
                    .filter(|&&c| {
                        self.ctx
                            .get::<FieldClass>(c)
                            .is_some_and(FieldClass::is_stored_instance)
                    })
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
                                .with_code("E320")
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
                                .with_code("E321")
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
                    by_ref: None,
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
                        // The declaration builder leaves a name it could not find empty.
                        field_name: hir_name(Some(f.field_name.clone()).filter(|n| !n.is_empty())),
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
fn lower_lit_pat(kind: &LitPat, span: &Span) -> HirLiteral {
    match kind {
        LitPat::Integer(s) => HirLiteral::Integer(parse_int(s)),
        LitPat::Float(s) => HirLiteral::Float(parse_float(s)),
        LitPat::String(s) => {
            let (value, escape_errors) =
                crate::literal::decode_string_literal_token(s, span.file_id, span.start);
            HirLiteral::String {
                value,
                escape_errors,
            }
        },
        LitPat::Bool(b) => HirLiteral::Bool(*b),
        LitPat::Char(s) => {
            let (value, escape_errors) = parse_char(s, span);
            HirLiteral::Char {
                value,
                escape_errors,
            }
        },
    }
}

/// Parse an integer literal string to `i128`.
///
/// `i128` holds every valid literal magnitude as a positive value (up to
/// `UInt64.maxValue = 2^64-1`), so unsigned maxima round-trip and the
/// `2^63`/`i64::MIN` bit-pattern collision (which used to hide out-of-range
/// `Int64` literals) cannot occur. Negation is applied separately by the
/// `negate` operator. Range/overflow checking is the range analyzer's job;
/// a magnitude beyond `i128` (absurdly long source) degrades to `0`.
pub(crate) fn parse_int(s: &str) -> i128 {
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
    i128::from_str_radix(body, radix).unwrap_or(0)
}

/// Parse a float literal string to f64.
pub(crate) fn parse_float(s: &str) -> f64 {
    s.replace('_', "").parse().unwrap_or(0.0)
}

/// Strip the surrounding quotes from a char-literal token.
///
/// `trim_matches` would strip ALL matching quotes, which breaks `'\''` by also
/// stripping the escaped quote content — so exactly one comes off each end.
fn char_literal_body(s: &str) -> &str {
    let inner = s.strip_prefix('\'').unwrap_or(s);
    inner.strip_suffix('\'').unwrap_or(inner)
}

/// Decode a char literal into its scalar value plus any escape errors.
///
/// Both positions — expression and pattern — go through this. They used to
/// differ: expressions called `parse_char_validated` (which `ctx.accumulate`d
/// uncoded diagnostics) while patterns called `parse_char` with no diagnostic
/// sink at all, so `'\u{D800}'` in a `match` arm silently became NUL while the
/// same literal in an expression errored (F26). Errors are now data on the
/// literal, exactly like `HirLiteral::String`, and `StringEscapeAnalyzer`
/// assigns E700-E703 to both.
pub(crate) fn parse_char(s: &str, span: &Span) -> (u32, Vec<EscapeError>) {
    let inner = char_literal_body(s);
    let (codepoints, errors) = unescape_char_content(inner, span);
    (codepoints.first().copied().unwrap_or(0), errors)
}

/// `parse_char` plus the arity checks that only make sense for a char literal
/// written in expression position: it must hold exactly one codepoint.
///
/// Those two remain `ctx.accumulate`d — they are not escape errors and have no
/// E-code of their own.
pub(crate) fn parse_char_validated(
    s: &str,
    span: &Span,
    ctx: &kestrel_hecs::QueryContext<'_>,
) -> (u32, Vec<EscapeError>) {
    let inner = char_literal_body(s);

    if inner.is_empty() {
        ctx.accumulate(
            kestrel_reporting::Diagnostic::error()
                .with_code("E709")
                .with_message("empty character literal")
                .with_labels(vec![
                    kestrel_reporting::Label::primary(span.file_id, span.range())
                        .with_message("character literal must contain exactly one codepoint"),
                ]),
        );
        return (0, Vec::new());
    }

    let (codepoints, errors) = unescape_char_content(inner, span);

    if codepoints.len() > 1 {
        ctx.accumulate(
            kestrel_reporting::Diagnostic::error()
                .with_code("E710")
                .with_message("character literal may only contain one codepoint")
                .with_labels(vec![
                    kestrel_reporting::Label::primary(span.file_id, span.range())
                        .with_message(format!("found {} codepoints", codepoints.len())),
                ]),
        );
    }

    (codepoints.first().copied().unwrap_or(0), errors)
}

/// Decode the body of a char literal into codepoints plus escape errors.
///
/// The escape TABLE is `kestrel_ast::escape` — shared with the string decoder,
/// so `\u` digit limits, `\x` range and unknown escapes have one answer. This
/// re-implemented it, and its `\u{` hex loop had neither a close-brace
/// requirement nor a digit limit, so `'\u{00000041}'` compiled to `'A'` while
/// `"\u{00000041}"` was rejected (F26).
///
/// Spans are relative to the literal's own span: the token text is the quoted
/// form, so an escape at body offset `k` sits at `span.start + 1 + k`.
fn unescape_char_content(s: &str, span: &Span) -> (Vec<u32>, Vec<EscapeError>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    let mut chars = s.char_indices().peekable();
    // +1 for the opening quote the body was stripped of.
    let body_start = span.start + 1;

    while let Some((i, c)) = chars.next() {
        if c != '\\' {
            result.push(c as u32);
            continue;
        }
        let escape_start = body_start + i;
        let decoded = decode_escape(&mut chars);
        let escape_span = Span::new(span.file_id, escape_start..escape_start + decoded.raw.len());

        match decoded.result {
            Ok(Escaped::Scalar(cp)) => result.push(cp),
            // Neither is meaningful inside a char literal.
            Ok(Escaped::LineContinuation) | Ok(Escaped::Interpolation) => {
                errors.push(EscapeError {
                    span: escape_span,
                    kind: EscapeErrorKind::InvalidEscape {
                        sequence: decoded.raw.clone(),
                    },
                });
            },
            Err(kind) => {
                errors.push(EscapeError {
                    span: escape_span,
                    kind,
                });
                // A malformed escape still has to yield a codepoint; 0 keeps
                // downstream code total, and the error is what the user sees.
                result.push(0);
            },
        }
    }

    (result, errors)
}
