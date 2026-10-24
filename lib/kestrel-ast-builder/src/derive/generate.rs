//! The Kestrel source each deriver writes. Semantics are in the attributes
//! design (§C5); the execution tests under `testdata/attributes/derive/` are
//! their specification.
//!
//! Every body reads fields through `self.f` / `other.f` or match bindings, and
//! calls the field type's own witness, so nothing here knows a field's type
//! beyond its text.

use std::fmt::Write;

use kestrel_syntax_tree::ast::{self, AstNode};

use super::DeriveKind;
use super::shape::{Case, Shape, ShapeBody, Slot};

/// How the generated extension names the type and its conditions.
pub(crate) enum Header {
    /// `@derive` on the type: `extend Path[T]: P where T: P`, with a bound
    /// for each parameter a relevant field's type mentions (design §C3).
    Type,
    /// `@derive` on an extension: its target and `where` clause, as written.
    Extension {
        target: String,
        where_clause: Option<String>,
    },
}

impl Header {
    pub(crate) fn for_site(item: &ast::Item) -> Header {
        match item {
            ast::Item::ExtensionDeclaration(ext) => Header::Extension {
                target: ext
                    .ty()
                    .map(|t| t.syntax().text().to_string())
                    .unwrap_or_default(),
                where_clause: ext.where_clause().map(|w| w.syntax().text().to_string()),
            },
            _ => Header::Type,
        }
    }

    fn write(&self, kind: DeriveKind, protocol: &str, shape: &Shape) -> String {
        match self {
            Header::Extension {
                target,
                where_clause,
            } => match where_clause {
                Some(w) => format!("extend {target}: {protocol} {w}"),
                None => format!("extend {target}: {protocol}"),
            },
            Header::Type => {
                let mut out = format!("extend {}", shape.path);
                if !shape.params.is_empty() {
                    let _ = write!(out, "[{}]", shape.params.join(", "));
                }
                let _ = write!(out, ": {protocol}");
                let slots = bound_slots(kind, shape);
                let mut constraints: Vec<String> = shape
                    .params
                    .iter()
                    .filter(|p| slots.iter().any(|s| s.idents.contains(p)))
                    .map(|p| format!("{p}: {protocol}"))
                    .collect();
                constraints.extend(shape.where_constraints.iter().cloned());
                if !constraints.is_empty() {
                    let _ = write!(out, " where {}", constraints.join(", "));
                }
                out
            },
        }
    }
}

/// The slots whose types need `P`: all of them, except that `Defaultable`
/// skips fields with an initializer and reads only the `@default` case.
fn bound_slots(kind: DeriveKind, shape: &Shape) -> Vec<&Slot> {
    if kind != DeriveKind::Defaultable {
        return shape.slots();
    }
    match &shape.body {
        ShapeBody::Struct(fields) => fields.iter().filter(|f| f.init.is_none()).collect(),
        ShapeBody::Enum(cases) => default_case(cases)
            .map(|c| c.payload.iter().collect())
            .unwrap_or_default(),
    }
}

/// The one case marked `@default`; `None` when there isn't exactly one (E957).
pub(crate) fn default_case(cases: &[Case]) -> Option<&Case> {
    let mut marked = cases.iter().filter(|c| c.is_default);
    let case = marked.next()?;
    marked.next().is_none().then_some(case)
}

/// The last segment of a type path as written: `a.B[T]` → `B`.
pub(crate) fn last_path_segment(text: &str) -> Option<String> {
    let head = text.split('[').next()?.trim();
    let last = head.rsplit('.').next()?.trim();
    (!last.is_empty()).then(|| last.to_string())
}

/// `extend ...: P { <witnesses> }` for one derived protocol, or `None` when
/// this shape can't derive it (an enum `Defaultable` without exactly one
/// `@default` case; the analyzer reports it).
pub(crate) fn extension(
    kind: DeriveKind,
    protocol: &str,
    header: &Header,
    shape: &Shape,
) -> Option<String> {
    let body = match kind {
        DeriveKind::Equatable => equatable(shape),
        DeriveKind::Hashable => hashable(shape),
        DeriveKind::Comparable => comparable(shape),
        DeriveKind::Formattable => formattable(shape),
        DeriveKind::Defaultable => defaultable(shape)?,
    };
    Some(format!(
        "{} {{\n{body}}}\n\n",
        header.write(kind, protocol, shape)
    ))
}

// ===== Patterns =====

/// `.Case` or `.Case(a0, a1)`; `_` for every binding when `prefix` is None.
fn case_pattern(case: &Case, prefix: Option<&str>) -> String {
    if case.payload.is_empty() {
        return format!(".{}", case.name);
    }
    let bindings: Vec<String> = (0..case.payload.len())
        .map(|i| match prefix {
            Some(p) => format!("{p}{i}"),
            None => "_".into(),
        })
        .collect();
    format!(".{}({})", case.name, bindings.join(", "))
}

/// `match <subject> { .A => 0, .B(_) => 1, }`: the case's declaration index.
fn case_index(subject: &str, cases: &[Case]) -> String {
    let mut out = format!("match {subject} {{\n");
    for (i, case) in cases.iter().enumerate() {
        let _ = writeln!(out, "            {} => {i},", case_pattern(case, None));
    }
    out.push_str("        }");
    out
}

/// For each field: (`self` side, `other` side). Struct fields are read in
/// place; enum payloads through the `a{i}` / `b{i}` bindings.
fn struct_pairs(fields: &[Slot]) -> Vec<(String, String)> {
    fields
        .iter()
        .map(|f| {
            let name = f.label.as_deref().unwrap_or_default();
            (format!("self.{name}"), format!("other.{name}"))
        })
        .collect()
}

fn case_pairs(case: &Case) -> Vec<(String, String)> {
    (0..case.payload.len())
        .map(|i| (format!("a{i}"), format!("b{i}")))
        .collect()
}

/// `match self { <arm(case)> ... }`, or `empty` for a case-less enum.
fn match_cases(cases: &[Case], empty: &str, arm: impl Fn(&Case) -> String) -> String {
    if cases.is_empty() {
        return empty.into();
    }
    let mut out = "match self {\n".to_string();
    for case in cases {
        let _ = writeln!(
            out,
            "            {} => {},",
            case_pattern(case, Some("a")),
            arm(case)
        );
    }
    out.push_str("        }");
    out
}

/// `match other { <case with b-bindings> => <same>, _ => <different>, }`.
/// The wildcard arm is dropped for a one-case enum, where it is unreachable.
fn match_other(case: &Case, cases: &[Case], same: &str, different: &str) -> String {
    let mut out = format!(
        "match other {{ {} => {same},",
        case_pattern(case, Some("b"))
    );
    if cases.len() > 1 {
        let _ = write!(out, " _ => {different},");
    }
    out.push_str(" }");
    out
}

// ===== Equatable =====

fn equal_all(pairs: &[(String, String)]) -> String {
    if pairs.is_empty() {
        return "true".into();
    }
    pairs
        .iter()
        .map(|(a, b)| format!("{a}.isEqual(to: {b})"))
        .collect::<Vec<_>>()
        .join(" and ")
}

fn equatable(shape: &Shape) -> String {
    let body = match &shape.body {
        ShapeBody::Struct(fields) => equal_all(&struct_pairs(fields)),
        ShapeBody::Enum(cases) => match_cases(cases, "true", |case| {
            match_other(case, cases, &equal_all(&case_pairs(case)), "false")
        }),
    };
    format!("    public func isEqual(to other: Self) -> Bool {{\n        {body}\n    }}\n")
}

// ===== Hashable =====

fn hashable(shape: &Shape) -> String {
    // The hasher's type parameter must not shadow one of the type's own.
    let h = (0..)
        .map(|i| {
            if i == 0 {
                "H".to_string()
            } else {
                format!("H{i}")
            }
        })
        .find(|n| !shape.params.contains(n))
        .unwrap_or_default();
    let feed = |pairs: Vec<(String, String)>| -> String {
        pairs
            .into_iter()
            .map(|(a, _)| format!("{a}.hash(into: hasher); "))
            .collect()
    };
    let body = match &shape.body {
        ShapeBody::Struct(fields) => feed(struct_pairs(fields)),
        // The case index first, so `.A(1)` and `.B(1)` hash apart.
        ShapeBody::Enum(cases) if cases.is_empty() => String::new(),
        ShapeBody::Enum(cases) => format!(
            "let index: Int64 = {};\n        index.hash(into: hasher);\n        {};",
            case_index("self", cases),
            match_cases(cases, "", |case| format!("{{ {}}}", feed(case_pairs(case))))
        ),
    };
    format!(
        "    public func hash[{h}](mutating into hasher: {h}) where {h}: Hasher {{\n        {body}\n    }}\n"
    )
}

// ===== Comparable =====

/// Fields lexicographically: the first `compare` that isn't `.Equal` decides.
/// Statements, then the tail expression.
fn compare_all(pairs: &[(String, String)]) -> String {
    let Some(((last_a, last_b), init)) = pairs.split_last() else {
        return "Ordering.Equal".into();
    };
    let mut out = String::new();
    for (i, (a, b)) in init.iter().enumerate() {
        let _ = write!(
            out,
            "let c{i}: Ordering = {a}.compare({b}); if c{i} != Ordering.Equal {{ return c{i} }} "
        );
    }
    let _ = write!(out, "{last_a}.compare({last_b})");
    out
}

fn comparable(shape: &Shape) -> String {
    let body = match &shape.body {
        ShapeBody::Struct(fields) => compare_all(&struct_pairs(fields)),
        ShapeBody::Enum(cases) if cases.is_empty() => "Ordering.Equal".into(),
        // Declaration order of the cases first, then the payloads.
        ShapeBody::Enum(cases) => format!(
            "let selfIndex: Int64 = {};\n        let otherIndex: Int64 = {};\n        \
             if selfIndex != otherIndex {{ return selfIndex.compare(otherIndex) }}\n        {}",
            case_index("self", cases),
            case_index("other", cases),
            match_cases(cases, "", |case| {
                // Braces make a match arm a block (elsewhere they'd be a closure).
                let same = format!("{{ {} }}", compare_all(&case_pairs(case)));
                match_other(case, cases, &same, "Ordering.Equal")
            })
        ),
    };
    format!("    public func compare(other: Self) -> Ordering {{\n        {body}\n    }}\n")
}

// ===== Formattable =====

/// `Name(a: 1, b: 2)`: labeled slots as `label: value`, unlabeled ones
/// positionally, each with default options. No slots prints `Name`.
///
/// The design (§C5) formats fields with `debug: true` so strings print
/// quoted. The stdlib doesn't support that yet: `String` ignores `debug`
/// and `Bool` renders as `Bool(true)` under it, so fields use the defaults.
fn format_slots(name: &str, values: &[(Option<&str>, String)]) -> String {
    if values.is_empty() {
        return format!("writer.append(\"{name}\"); ");
    }
    let mut out = String::new();
    for (i, (label, value)) in values.iter().enumerate() {
        let open = if i == 0 {
            format!("{name}(")
        } else {
            ", ".into()
        };
        let label = label.map(|l| format!("{l}: ")).unwrap_or_default();
        let _ = write!(
            out,
            "writer.append(\"{open}{label}\"); {value}.format(into: writer, fieldOptions); "
        );
    }
    out.push_str("writer.append(\")\"); ");
    out
}

fn formattable(shape: &Shape) -> String {
    let body = match &shape.body {
        ShapeBody::Struct(fields) => {
            let values: Vec<_> = fields
                .iter()
                .map(|f| {
                    let label = f.label.as_deref();
                    (label, format!("self.{}", label.unwrap_or_default()))
                })
                .collect();
            format_slots(&shape.name, &values)
        },
        ShapeBody::Enum(cases) => {
            let arms = match_cases(cases, "", |case| {
                let values: Vec<_> = case
                    .payload
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (p.label.as_deref(), format!("a{i}")))
                    .collect();
                format!("{{ {}}}", format_slots(&case.name, &values))
            });
            if arms.is_empty() {
                arms
            } else {
                format!("{arms};")
            }
        },
    };
    // Width and padding apply to the whole value: render it unpadded, then pad.
    format!(
        "    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {{
        if let .Some(_) = options.width {{
            var unpadded = options;
            unpadded.width = .None;
            _writePadded(into: writer, self.formatted(unpadded), options);
            return
        }}
        let fieldOptions = FormatOptions.default();
        {body}
    }}\n"
    )
}

// ===== Defaultable =====

/// A slot's default: its initializer, else `Ty()`. `None` for a type that
/// isn't a path (a tuple, an optional): none of those is `Defaultable`, and
/// `DeriveSite` reports the slot (E952).
fn default_value(slot: &Slot) -> Option<String> {
    if let Some(init) = &slot.init {
        return Some(init.trim().to_string());
    }
    slot.ty_is_path.then(|| format!("{}()", slot.ty))
}

fn defaultable(shape: &Shape) -> Option<String> {
    let body = match &shape.body {
        ShapeBody::Struct(fields) => fields
            .iter()
            .map(|f| {
                let name = f.label.as_deref().unwrap_or_default();
                Some(format!("self.{name} = {}; ", default_value(f)?))
            })
            .collect::<Option<String>>()?,
        ShapeBody::Enum(cases) => {
            let case = default_case(cases)?;
            let args = case
                .payload
                .iter()
                .map(|p| {
                    let value = default_value(p)?;
                    Some(match &p.label {
                        Some(l) => format!("{l}: {value}"),
                        None => value,
                    })
                })
                .collect::<Option<Vec<String>>>()?;
            if args.is_empty() {
                format!("self = .{}; ", case.name)
            } else {
                format!("self = .{}({}); ", case.name, args.join(", "))
            }
        },
    };
    Some(format!("    public init() {{\n        {body}\n    }}\n"))
}
