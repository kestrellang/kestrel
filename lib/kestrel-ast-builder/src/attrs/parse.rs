//! Argument parsing: raw [`AstAttribute`]s in, typed [`ParsedAttr`]s and
//! [`AttrError`]s out.
//!
//! This is the one function that interprets attribute text. It never reports
//! anything itself: errors are data, reported by the `AttributeValidation`
//! analyzer. A malformed attribute is left out of the typed list, so
//! consumers never see it.

use kestrel_span::Span;

use super::registry::{self, AttrDef, ParamDef, ParamKind};
use super::target::AttrTarget;
use super::value::{AttrEntry, AttrError, AttrValue, ParsedAttr};
use crate::components::{AstAttribute, AstAttributeArg, AttrLiteral};

/// Parse every attribute of one declaration. `target` is `None` for entities
/// that never carry attributes; `has_body` drives `needs_body` (E905).
pub fn parse_attributes(
    attrs: &[AstAttribute],
    target: Option<AttrTarget>,
    has_body: bool,
    decl_span: &Span,
) -> (Vec<AttrEntry>, Vec<AttrError>) {
    let mut entries: Vec<AttrEntry> = Vec::new();
    let mut errors = Vec::new();
    // Names seen so far, with the span of their first use, for E901/E906.
    let mut seen: Vec<(&'static AttrDef, Span)> = Vec::new();

    for attr in attrs {
        let Some(def) = registry::lookup(&attr.name) else {
            errors.push(unknown_attribute(attr));
            continue;
        };
        if let Some(target) = target
            && !def.targets.contains(target)
        {
            errors.push(AttrError::new(
                "E900",
                format!(
                    "'@{}' cannot be applied to {}; it applies to {}",
                    def.name,
                    target.describe(),
                    def.targets.describe()
                ),
                "attribute not allowed here",
                attr.span.clone(),
            ));
            continue;
        }
        if !def.repeatable
            && let Some((_, first)) = seen.iter().find(|(d, _)| d.name == def.name)
        {
            let message = match def.repeat_hint {
                Some(hint) => format!("'@{}' appears twice; {hint}", def.name),
                None => format!("duplicate attribute '@{}'", def.name),
            };
            errors.push(
                AttrError::new("E901", message, "repeated here", attr.span.clone())
                    .with_secondary(first.clone(), "first written here"),
            );
            continue;
        }
        if let Some((conflict, other_span)) = find_conflict(def, &seen) {
            errors.push(
                AttrError::new("E906", conflict, "conflicting attribute", attr.span.clone())
                    .with_secondary(other_span, "conflicts with this"),
            );
            continue;
        }
        seen.push((def, attr.span.clone()));
        // A conflicting partner explains the missing body better (`@inline`
        // on an `@extern`), and is reported on whichever comes second.
        if def.needs_body && !has_body && !conflicts_with_any(def, attrs) {
            errors.push(AttrError::new(
                "E905",
                format!(
                    "'@{}' needs a body; {} has none",
                    def.name,
                    target.map_or("this declaration", |t| t.describe())
                ),
                "declaration has no body",
                attr.span.clone(),
            ));
            continue;
        }

        match match_args(def, attr, decl_span) {
            Ok(args) => {
                errors.extend(spelling_warnings(def, attr));
                if let Some(e) = value_error(def, attr, &args) {
                    errors.push(e);
                    continue;
                }
                if let Some(parsed) = build(def, &args) {
                    entries.push(AttrEntry {
                        attr: parsed,
                        span: attr.span.clone(),
                    });
                }
            },
            Err(mut errs) => errors.append(&mut errs),
        }
    }
    errors.extend(packed_with_align(&mut entries));
    (entries, errors)
}

/// The largest `@align`: what `posix_memalign` and the stack provide without
/// a special path.
const MAX_ALIGN: i128 = 4096;

/// A well-typed argument whose value is out of range.
fn value_error(def: &AttrDef, attr: &AstAttribute, args: &Args) -> Option<AttrError> {
    if def.name != "align" {
        return None;
    }
    let AttrValue::Int(n) = args.get(0)? else {
        return None;
    };
    if (1..=MAX_ALIGN).contains(n) && n.count_ones() == 1 {
        return None;
    }
    Some(AttrError::new(
        "E909",
        format!("'@align({n})' must be a power of two between 1 and {MAX_ALIGN}"),
        "invalid alignment",
        attr.args
            .first()
            .map_or_else(|| attr.span.clone(), |a| a.span.clone()),
    ))
}

/// `@layout(.Packed)` with `@align` (E906). Not a registry conflict, because
/// `@layout(.C)` with `@align` is fine. Reported on whichever comes second,
/// which is dropped like any other rejected attribute.
fn packed_with_align(entries: &mut Vec<AttrEntry>) -> Option<AttrError> {
    let packed = entries.iter().position(|e| {
        matches!(
            e.attr,
            ParsedAttr::Layout(super::value::LayoutOption::Packed)
        )
    })?;
    let align = entries
        .iter()
        .position(|e| matches!(e.attr, ParsedAttr::Align(_)))?;
    let (first, second) = (packed.min(align), packed.max(align));
    let dropped = entries.remove(second);
    Some(
        AttrError::new(
            "E906",
            "'@layout(.Packed)' cannot be combined with '@align': a packed type has alignment 1"
                .into(),
            "conflicting attribute",
            dropped.span,
        )
        .with_secondary(entries[first].span.clone(), "conflicts with this"),
    )
}

/// Parse one attribute's arguments alone, with no target, repeat or conflict
/// checks. For the `Build` phase (`@platform` filtering), which runs before
/// entities exist; errors are reported later through `AttrsOf`.
pub fn parse_standalone(attr: &AstAttribute) -> Option<ParsedAttr> {
    let def = registry::lookup(&attr.name)?;
    let args = match_args(def, attr, &attr.span).ok()?;
    build(def, &args)
}

/// Warnings for spellings that are accepted for now but going away.
fn spelling_warnings(def: &AttrDef, attr: &AstAttribute) -> Vec<AttrError> {
    if def.name != "platform" {
        return Vec::new();
    }
    attr.args
        .iter()
        .filter_map(|arg| match &arg.value {
            AttrLiteral::Case(c) if c.starts_with(char::is_lowercase) => {
                let mut fixed = c.clone();
                fixed[..1].make_ascii_uppercase();
                Some(
                    AttrError::new(
                        "E914",
                        format!("'.{c}' is the old spelling; write '.{fixed}'"),
                        "old platform spelling",
                        arg.span.clone(),
                    )
                    .as_warning(),
                )
            },
            _ => None,
        })
        .collect()
}

/// Whether any attribute on the declaration conflicts with `def`.
fn conflicts_with_any(def: &AttrDef, attrs: &[AstAttribute]) -> bool {
    attrs
        .iter()
        .filter_map(|a| registry::lookup(&a.name))
        .any(|other| {
            def.conflicts.iter().any(|c| c.with == other.name)
                || other.conflicts.iter().any(|c| c.with == def.name)
        })
}

/// The conflict message if `def` conflicts with an attribute already seen
/// (declared on either side of the pair).
fn find_conflict(def: &AttrDef, seen: &[(&'static AttrDef, Span)]) -> Option<(String, Span)> {
    seen.iter().find_map(|(other, span)| {
        let reason = def
            .conflicts
            .iter()
            .find(|c| c.with == other.name)
            .or_else(|| other.conflicts.iter().find(|c| c.with == def.name))?
            .reason;
        Some((
            format!(
                "'@{}' cannot be combined with '@{}': {reason}",
                def.name, other.name
            ),
            span.clone(),
        ))
    })
}

fn unknown_attribute(attr: &AstAttribute) -> AttrError {
    let message = match suggest(&attr.name, registry::names()) {
        Some(s) => format!("unknown attribute '{}'; did you mean '@{s}'?", attr.name),
        None => format!("unknown attribute '{}'", attr.name),
    };
    AttrError::new("E461", message, "unknown attribute", attr.span.clone())
}

/// The matched arguments of one attribute, indexed like `def.params`. A
/// variadic parameter holds every value it took.
pub(super) struct Args {
    values: Vec<Vec<AttrValue>>,
}

impl Args {
    /// The single value of parameter `i`, if given.
    pub(super) fn get(&self, i: usize) -> Option<&AttrValue> {
        self.values.get(i).and_then(|v| v.first())
    }

    /// Every value of (variadic) parameter `i`.
    pub(super) fn all(&self, i: usize) -> &[AttrValue] {
        self.values.get(i).map_or(&[], |v| v.as_slice())
    }

    pub(super) fn str(&self, i: usize) -> Option<String> {
        match self.get(i)? {
            AttrValue::Str(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub(super) fn case(&self, i: usize) -> Option<&str> {
        match self.get(i)? {
            AttrValue::Case(c) => Some(c),
            _ => None,
        }
    }
}

/// Match arguments to parameters with the function-argument rules:
/// positional first, then labelled in any order.
fn match_args(
    def: &AttrDef,
    attr: &AstAttribute,
    decl_span: &Span,
) -> Result<Args, Vec<AttrError>> {
    // An `Any` parameter (`@dummy`) takes every argument, labelled or not.
    if let [param] = def.params
        && matches!(param.kind, ParamKind::Any)
    {
        let values = attr.args.iter().map(|a| any_value(&a.value)).collect();
        return Ok(Args {
            values: vec![values],
        });
    }
    let mut values: Vec<Vec<AttrValue>> = vec![Vec::new(); def.params.len()];
    let mut errors = Vec::new();
    let positional: Vec<usize> = (0..def.params.len())
        .filter(|&i| def.params[i].label.is_none())
        .collect();
    let mut next_positional = 0;
    let mut seen_label = false;
    let mut given = vec![false; def.params.len()];
    // An unknown label is likely the missing argument misspelled; don't
    // report it twice.
    let mut bad_label = false;

    for arg in &attr.args {
        let index = match &arg.label {
            None => {
                if seen_label {
                    // Still bind it below, so the misplaced argument isn't
                    // also reported as missing.
                    errors.push(AttrError::new(
                        "E904",
                        "positional argument after a labeled one".into(),
                        "move this before the labeled arguments",
                        arg.span.clone(),
                    ));
                }
                let Some(&i) = positional.get(next_positional) else {
                    errors.push(too_many(def, arg));
                    continue;
                };
                if !def.params[i].variadic {
                    next_positional += 1;
                }
                i
            },
            Some(label) => {
                seen_label = true;
                let Some(i) = def.params.iter().position(|p| p.label == Some(label)) else {
                    errors.push(unknown_label(def, arg, label));
                    bad_label = true;
                    continue;
                };
                if given[i] {
                    errors.push(AttrError::new(
                        "E904",
                        format!("argument '{label}' given twice"),
                        "repeated argument",
                        arg.label_span.clone().unwrap_or_else(|| arg.span.clone()),
                    ));
                    continue;
                }
                i
            },
        };
        given[index] = true;
        match convert(&def.params[index], arg) {
            Ok(v) => values[index].push(v),
            Err(e) => errors.push(e),
        }
    }

    for (i, param) in def.params.iter().enumerate() {
        if !param.required || given[i] || bad_label {
            continue;
        }
        let message = format!("'@{}' needs {}", def.name, param.what);
        errors.push(match param.missing_code {
            // A code that predates the registry keeps its old location (the
            // declaration), so existing diagnostics and tests stay put.
            Some(code) => AttrError::new(code, message, "missing argument", decl_span.clone())
                .with_secondary(attr.span.clone(), "attribute written here"),
            None => AttrError::new("E902", message, "missing argument", attr.span.clone()),
        });
    }

    if errors.is_empty() {
        Ok(Args { values })
    } else {
        Err(errors)
    }
}

fn too_many(def: &AttrDef, arg: &AstAttributeArg) -> AttrError {
    let n = def.params.iter().filter(|p| p.label.is_none()).count();
    let message = match n {
        0 => format!("'@{}' takes no positional arguments", def.name),
        1 => format!("'@{}' takes at most one argument", def.name),
        n => format!("'@{}' takes at most {n} positional arguments", def.name),
    };
    AttrError::new("E907", message, "unexpected argument", arg.span.clone())
}

fn unknown_label(def: &AttrDef, arg: &AstAttributeArg, label: &str) -> AttrError {
    let labels = def.params.iter().filter_map(|p| p.label);
    let message = match suggest(label, labels) {
        Some(s) => format!(
            "'@{}' has no argument '{label}'; did you mean '{s}'?",
            def.name
        ),
        None => format!("'@{}' has no argument '{label}'", def.name),
    };
    AttrError::new(
        "E904",
        message,
        "unknown argument",
        arg.label_span.clone().unwrap_or_else(|| arg.span.clone()),
    )
}

/// Convert one argument's literal to the parameter's kind.
#[allow(clippy::result_large_err)]
fn convert(param: &ParamDef, arg: &AstAttributeArg) -> Result<AttrValue, AttrError> {
    let wrong = |expected: &str| {
        AttrError::new(
            "E903",
            format!("expected {expected}, found '{}'", arg.value.display()),
            "wrong kind of argument",
            arg.span.clone(),
        )
    };
    match (&param.kind, &arg.value) {
        (_, AttrLiteral::Missing) => Err(wrong("an argument")),
        (ParamKind::Any, lit) => Ok(any_value(lit)),
        (ParamKind::Str, AttrLiteral::Str(raw)) => unescape(raw)
            .map(AttrValue::Str)
            .ok_or_else(|| wrong("a valid string literal")),
        (ParamKind::Str, _) => Err(wrong("a string literal")),
        (ParamKind::Int, AttrLiteral::Int(raw)) => parse_int(raw)
            .map(AttrValue::Int)
            .ok_or_else(|| wrong("an integer literal")),
        (ParamKind::Int, _) => Err(wrong("an integer literal")),
        (ParamKind::Bool, AttrLiteral::Bool(b)) => Ok(AttrValue::Bool(*b)),
        (ParamKind::Bool, _) => Err(wrong("'true' or 'false'")),
        (ParamKind::Cases(noun, cases), AttrLiteral::Case(c)) => {
            if cases.contains(&c.as_str()) {
                Ok(AttrValue::Case(c.clone()))
            } else {
                Err(AttrError::new(
                    "E903",
                    format!(
                        "'.{c}' is not {} {noun}; expected {}",
                        article(noun),
                        list_cases(cases)
                    ),
                    format!("unknown {noun}"),
                    arg.span.clone(),
                ))
            }
        },
        (ParamKind::Cases(noun, cases), _) => Err(wrong(&format!(
            "{} {noun} ({})",
            article(noun),
            list_cases(cases)
        ))),
        (ParamKind::OpenCase(noun, valid), AttrLiteral::Case(c)) => {
            if valid(c) {
                Ok(AttrValue::Case(c.clone()))
            } else {
                Err(AttrError::new(
                    "E903",
                    format!("unknown {noun} '.{c}'"),
                    format!("unknown {noun}"),
                    arg.span.clone(),
                ))
            }
        },
        (ParamKind::OpenCase(noun, _), _) => Err(wrong(&format!("a {noun} like '.Name'"))),
        (ParamKind::Path, AttrLiteral::Path(p)) => Ok(AttrValue::Path(p.clone())),
        (ParamKind::Path, _) => Err(wrong("a name")),
    }
}

/// "a" or "an" before `noun`.
fn article(noun: &str) -> &'static str {
    if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

/// `'.Always' or '.Never'`.
fn list_cases(cases: &[&str]) -> String {
    let quoted: Vec<_> = cases.iter().map(|c| format!("'.{c}'")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} or {last}", init.join(", ")),
    }
}

/// `@dummy` accepts anything; keep its shape for completeness.
fn any_value(lit: &AttrLiteral) -> AttrValue {
    match lit {
        AttrLiteral::Str(raw) => AttrValue::Str(unescape(raw).unwrap_or_default()),
        AttrLiteral::Int(raw) => AttrValue::Int(parse_int(raw).unwrap_or_default()),
        AttrLiteral::Float(raw) => AttrValue::Float(raw.clone()),
        AttrLiteral::Bool(b) => AttrValue::Bool(*b),
        AttrLiteral::Case(c) => AttrValue::Case(c.clone()),
        AttrLiteral::Path(p) => AttrValue::Path(p.clone()),
        AttrLiteral::Missing => AttrValue::Bool(false),
    }
}

/// Decode a string literal (with its quotes) through the compiler's one
/// escape table. `None` for a malformed literal or an interpolation.
fn unescape(raw: &str) -> Option<String> {
    use kestrel_ast::escape::{Escaped, decode_escape};
    let inner = raw.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match decode_escape(&mut chars).result.ok()? {
            Escaped::Scalar(v) => out.push(char::from_u32(v)?),
            Escaped::LineContinuation => {},
            Escaped::Interpolation => return None,
        }
    }
    Some(out)
}

/// Parse an integer literal: decimal, `0x`, `0o`, `0b`, with `_` separators.
fn parse_int(raw: &str) -> Option<i128> {
    let s: String = raw.chars().filter(|&c| c != '_').collect();
    let (digits, radix) = match s.get(..2) {
        Some("0x" | "0X") => (&s[2..], 16),
        Some("0o" | "0O") => (&s[2..], 8),
        Some("0b" | "0B") => (&s[2..], 2),
        _ => (s.as_str(), 10),
    };
    i128::from_str_radix(digits, radix).ok()
}

/// The closest candidate for "did you mean": within a small edit distance
/// (a swap of two letters counts once), or one name a prefix of the other
/// (`mangle` → `mangleName`).
pub fn suggest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let max = (name.len() / 3).max(1);
    candidates
        .map(|c| {
            let prefix = name.len() >= 3 && (c.starts_with(name) || name.starts_with(c));
            (if prefix { 0 } else { edit_distance(name, c) }, c)
        })
        .filter(|&(d, _)| d <= max)
        .min_by_key(|&(d, _)| d)
        .map(|(_, c)| c)
}

/// Optimal-string-alignment distance: Levenshtein plus adjacent swaps.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// Build the typed value of a well-formed attribute. One arm per registry
/// entry; `None` only for an entry whose arguments were already rejected.
fn build(def: &AttrDef, args: &Args) -> Option<ParsedAttr> {
    use super::value::{Abi, DocOption, InlineMode, LayoutOption, Restriction, RestrictionKind};
    use crate::components::Os;
    Some(match def.name {
        "align" => match args.get(0)? {
            AttrValue::Int(n) => ParsedAttr::Align(u64::try_from(*n).ok()?),
            _ => return None,
        },
        "builtin" => {
            let name = args.case(0)?;
            ParsedAttr::Builtin(
                kestrel_hir::Builtin::from_attribute_name(name)?,
                name.to_string(),
            )
        },
        "default" => ParsedAttr::Default,
        "derive" => ParsedAttr::Derive(
            args.all(0)
                .iter()
                .filter_map(|v| match v {
                    AttrValue::Path(p) => Some(p.clone()),
                    _ => None,
                })
                .collect(),
        ),
        "deprecated" => ParsedAttr::Restricted(Restriction {
            kind: RestrictionKind::Deprecated,
            message: args.str(0),
            since: args.str(1),
            replacement: args.str(2),
        }),
        "doc" => ParsedAttr::Doc(match args.case(0)? {
            "Hidden" => DocOption::Hidden,
            _ => return None,
        }),
        "dummy" => ParsedAttr::Dummy,
        "nonExhaustive" => ParsedAttr::NonExhaustive,
        "observabledrop" => ParsedAttr::ObservableDrop,
        "extern" => ParsedAttr::Extern {
            abi: match args.case(0)? {
                "C" => Abi::C,
                _ => return None,
            },
            mangle_name: args.str(1),
        },
        "fileconstant" => ParsedAttr::FileConstant { path: args.str(0)? },
        "layout" => ParsedAttr::Layout(match args.case(0)? {
            "C" => LayoutOption::C,
            "Packed" => LayoutOption::Packed,
            _ => return None,
        }),
        "main" => ParsedAttr::Main,
        "inline" => ParsedAttr::Inline(match args.case(0) {
            None => InlineMode::Hint,
            Some("Always") => InlineMode::Always,
            Some("Never") => InlineMode::Never,
            Some(_) => return None,
        }),
        "platform" => ParsedAttr::Platform(
            args.all(0)
                .iter()
                .filter_map(|v| match v {
                    AttrValue::Case(c) => Os::from_name(c),
                    _ => None,
                })
                .collect(),
        ),
        "unavailable" => ParsedAttr::Restricted(Restriction {
            kind: RestrictionKind::Unavailable,
            message: args.str(0),
            since: None,
            replacement: args.str(1),
        }),
        _ => return None,
    })
}
