//! Source generation from `kinds.txt` + `kestrel.ungram`.
//!
//! Produces three checked-in files and fails when any is stale:
//!
//! | File | Contents |
//! |------|----------|
//! | `src/generated/kinds.rs` | `SyntaxKind`, `SyntaxKind::ALL`, `From<Token>` |
//! | `src/ast/generated.rs` | typed views: one struct per node, one enum per union |
//! | `src/validate/generated.rs` | each node's rule, for the conformance validator |
//!
//! Run with `UPDATE_GENERATED=1` to rewrite them.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ungrammar::{Grammar, Rule};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Class {
    Node,
    Token,
    Trivia,
    Special,
}

#[derive(Debug, Clone)]
struct KindDef {
    name: String,
    class: Class,
    spelling: Option<String>,
    doc: Option<String>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read_kinds() -> Vec<KindDef> {
    let text = std::fs::read_to_string(root().join("kinds.txt")).unwrap();
    let mut kinds = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (body, doc) = match line.find(" # ") {
            Some(i) => (&line[..i], Some(line[i + 3..].trim().to_string())),
            None => (line, None),
        };
        let mut parts = body.split_whitespace();
        let class = match parts.next().unwrap() {
            "node" => Class::Node,
            "token" => Class::Token,
            "trivia" => Class::Trivia,
            "special" => Class::Special,
            other => panic!("unknown kind class {other:?}"),
        };
        let name = parts.next().unwrap().to_string();
        let spelling = parts.next().map(|s| s.trim_matches('\'').to_string());
        kinds.push(KindDef {
            name,
            class,
            spelling,
            doc,
        });
    }
    kinds
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn plural(name: &str) -> String {
    if name == "ty" {
        return "types".to_string();
    }
    if name.ends_with('y') && !name.ends_with("ey") {
        format!("{}ies", &name[..name.len() - 1])
    } else if name.ends_with('s') || name.ends_with('x') {
        format!("{name}es")
    } else {
        format!("{name}s")
    }
}

// ----- model ------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Field {
    Node {
        name: String,
        ty: String,
        many: bool,
        nth: usize,
    },
    Token {
        name: String,
        kind: String,
        nth: usize,
    },
}

impl Field {
    fn name(&self) -> &str {
        match self {
            Field::Node { name, .. } | Field::Token { name, .. } => name,
        }
    }
}

struct Model<'g> {
    grammar: &'g Grammar,
    kinds: Vec<KindDef>,
    /// spelling → kind name
    tokens: HashMap<String, String>,
    node_kinds: BTreeSet<String>,
    /// union name → member node kinds (flattened)
    enums: BTreeMap<String, Vec<String>>,
    /// node name → its rule
    nodes: BTreeMap<String, &'g Rule>,
    node_order: Vec<String>,
}

impl<'g> Model<'g> {
    fn new(grammar: &'g Grammar, kinds: Vec<KindDef>) -> Self {
        let tokens = kinds
            .iter()
            .filter_map(|k| k.spelling.clone().map(|s| (s, k.name.clone())))
            .collect::<HashMap<_, _>>();
        let node_kinds = kinds
            .iter()
            .filter(|k| k.class == Class::Node)
            .map(|k| k.name.clone())
            .collect::<BTreeSet<_>>();
        let mut model = Model {
            grammar,
            kinds,
            tokens,
            node_kinds,
            enums: BTreeMap::new(),
            nodes: BTreeMap::new(),
            node_order: Vec::new(),
        };
        let mut raw_enums = BTreeMap::new();
        for node in grammar.iter() {
            let data = &grammar[node];
            if model.node_kinds.contains(&data.name) {
                model.nodes.insert(data.name.clone(), &data.rule);
                model.node_order.push(data.name.clone());
            } else {
                let Rule::Alt(alts) = &data.rule else {
                    panic!(
                        "`{}` is not a SyntaxKind, so it must be a union of nodes",
                        data.name
                    );
                };
                let members = alts
                    .iter()
                    .map(|a| match a {
                        Rule::Node(n) => grammar[*n].name.clone(),
                        other => panic!("union `{}` has a non-node member {other:?}", data.name),
                    })
                    .collect::<Vec<_>>();
                raw_enums.insert(data.name.clone(), members);
            }
        }
        // Flatten unions of unions.
        for name in raw_enums.keys().cloned().collect::<Vec<_>>() {
            let mut out = Vec::new();
            flatten(&raw_enums, &name, &mut out);
            model.enums.insert(name, out);
        }
        model
    }

    fn token_kind(&self, spelling: &str) -> String {
        self.tokens
            .get(spelling)
            .unwrap_or_else(|| panic!("token '{spelling}' is not spelled in kinds.txt"))
            .clone()
    }

    /// Accessors for a node, in first-appearance order.
    fn fields(&self, rule: &Rule) -> Vec<Field> {
        let mut acc: Vec<Field> = Vec::new();
        let mut seen: HashMap<String, usize> = HashMap::new();
        self.lower(rule, None, false, &mut acc, &mut seen);
        // Unlabeled duplicates of one node type become one `many` field.
        let mut out: Vec<Field> = Vec::new();
        for f in acc {
            if let Some(existing) = out.iter_mut().find(|e| e.name() == f.name()) {
                if let (Field::Node { many, .. }, Field::Node { .. }) = (existing, &f) {
                    *many = true;
                }
                continue;
            }
            out.push(f);
        }
        // A `many` field whose name is still singular gets pluralised.
        for f in &mut out {
            if let Field::Node {
                name,
                many: true,
                nth: 0,
                ty,
            } = f
                && *name == snake(ty)
            {
                *name = plural(name);
            }
        }
        out
    }

    fn lower(
        &self,
        rule: &Rule,
        label: Option<&str>,
        many: bool,
        acc: &mut Vec<Field>,
        seen: &mut HashMap<String, usize>,
    ) {
        match rule {
            Rule::Labeled { label, rule } => self.lower(rule, Some(label), many, acc, seen),
            Rule::Node(n) => {
                let ty = self.grammar[*n].name.clone();
                let nth = *seen.get(&ty).unwrap_or(&0);
                *seen.entry(ty.clone()).or_default() += 1;
                acc.push(Field::Node {
                    name: label.map(str::to_string).unwrap_or_else(|| snake(&ty)),
                    ty,
                    many,
                    nth: if label.is_some() { nth } else { 0 },
                });
            },
            Rule::Token(t) => {
                let kind = self.token_kind(&self.grammar[*t].name);
                let nth = *seen.get(&kind).unwrap_or(&0);
                *seen.entry(kind.clone()).or_default() += 1;
                acc.push(Field::Token {
                    name: label
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("{}_token", snake(&kind))),
                    nth: if label.is_some() { nth } else { 0 },
                    kind,
                });
            },
            Rule::Seq(rules) | Rule::Alt(rules) => {
                for r in rules {
                    self.lower(r, label, many, acc, seen);
                }
            },
            Rule::Opt(r) => self.lower(r, label, many, acc, seen),
            Rule::Rep(r) => self.lower(r, label, true, acc, seen),
        }
    }
}

fn flatten(enums: &BTreeMap<String, Vec<String>>, name: &str, out: &mut Vec<String>) {
    for m in &enums[name] {
        if enums.contains_key(m) {
            flatten(enums, m, out);
        } else if !out.contains(m) {
            out.push(m.clone());
        }
    }
}

// ----- generation ------------------------------------------------------------

const HEADER: &str = "//! @generated by `tests/sourcegen.rs` from `kinds.txt` and `kestrel.ungram`.\n//! Do not edit by hand: change the sources and run\n//! `UPDATE_GENERATED=1 cargo test -p kestrel-syntax-tree --test sourcegen`.\n\n";

fn gen_kinds(kinds: &[KindDef]) -> String {
    let mut s = String::from(HEADER);
    s.push_str("use kestrel_lexer::Token;\n\n");
    s.push_str("/// Every node and token kind of the Kestrel CST.\n///\n/// The discriminant order is `kinds.txt`'s line order, which is append-only:\n/// rowan stores kinds as raw `u16`s.\n");
    s.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub enum SyntaxKind {\n");
    for k in kinds {
        if let Some(doc) = &k.doc {
            writeln!(s, "    /// {doc}").unwrap();
        }
        writeln!(s, "    {},", k.name).unwrap();
    }
    s.push_str("    /// Not a syntax kind — the end-of-enum marker. Its discriminant is the\n    /// variant count, which lets `syntax_kind_table_round_trips` prove `ALL`\n    /// complete. Never construct it.\n    #[doc(hidden)]\n    __NotAKind,\n}\n\n");
    s.push_str("impl SyntaxKind {\n    /// Every variant, in declaration order: `ALL[n] as u16 == n`.\n    pub const ALL: &'static [SyntaxKind] = &[\n");
    for k in kinds {
        writeln!(s, "        SyntaxKind::{},", k.name).unwrap();
    }
    s.push_str("    ];\n}\n\n");
    s.push_str("impl From<Token> for SyntaxKind {\n    fn from(token: Token) -> Self {\n        match token {\n");
    for k in kinds {
        if matches!(k.class, Class::Token | Class::Trivia) {
            writeln!(s, "            Token::{0} => SyntaxKind::{0},", k.name).unwrap();
        }
    }
    s.push_str("        }\n    }\n}\n");
    s
}

fn gen_nodes(model: &Model<'_>) -> String {
    let mut s = String::from(HEADER);
    s.push_str("#![allow(clippy::all)]\n\nuse crate::ast::{AstChildren, AstNode, support};\nuse crate::{SyntaxKind, SyntaxNode, SyntaxToken};\n\n");
    for name in &model.node_order {
        let rule = model.nodes[name];
        writeln!(s, "/// `{name}` node.").unwrap();
        writeln!(
            s,
            "#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub struct {name} {{\n    pub(crate) syntax: SyntaxNode,\n}}\n"
        )
        .unwrap();
        writeln!(s, "impl {name} {{").unwrap();
        for f in model.fields(rule) {
            match f {
                Field::Node {
                    name: fname,
                    ty,
                    many: true,
                    ..
                } => writeln!(
                    s,
                    "    pub fn {fname}(&self) -> AstChildren<{ty}> {{\n        support::children(&self.syntax)\n    }}"
                )
                .unwrap(),
                Field::Node {
                    name: fname,
                    ty,
                    nth: 0,
                    ..
                } => writeln!(
                    s,
                    "    pub fn {fname}(&self) -> Option<{ty}> {{\n        support::child(&self.syntax)\n    }}"
                )
                .unwrap(),
                Field::Node {
                    name: fname,
                    ty,
                    nth,
                    ..
                } => writeln!(
                    s,
                    "    pub fn {fname}(&self) -> Option<{ty}> {{\n        support::children(&self.syntax).nth({nth})\n    }}"
                )
                .unwrap(),
                Field::Token {
                    name: fname,
                    kind,
                    nth: 0,
                } => writeln!(
                    s,
                    "    pub fn {fname}(&self) -> Option<SyntaxToken> {{\n        support::token(&self.syntax, SyntaxKind::{kind})\n    }}"
                )
                .unwrap(),
                Field::Token {
                    name: fname,
                    kind,
                    nth,
                } => writeln!(
                    s,
                    "    pub fn {fname}(&self) -> Option<SyntaxToken> {{\n        support::nth_token(&self.syntax, SyntaxKind::{kind}, {nth})\n    }}"
                )
                .unwrap(),
            }
        }
        writeln!(s, "}}\n").unwrap();
        writeln!(
            s,
            "impl AstNode for {name} {{\n    fn can_cast(kind: SyntaxKind) -> bool {{\n        kind == SyntaxKind::{name}\n    }}\n    fn cast(syntax: SyntaxNode) -> Option<Self> {{\n        Self::can_cast(syntax.kind()).then_some(Self {{ syntax }})\n    }}\n    fn syntax(&self) -> &SyntaxNode {{\n        &self.syntax\n    }}\n}}\n"
        )
        .unwrap();
    }
    for (name, members) in &model.enums {
        writeln!(s, "/// One of: {}.", members.join(", ")).unwrap();
        writeln!(
            s,
            "#[derive(Debug, Clone, PartialEq, Eq, Hash)]\npub enum {name} {{"
        )
        .unwrap();
        for m in members {
            writeln!(s, "    {m}({m}),").unwrap();
        }
        writeln!(s, "}}\n").unwrap();
        writeln!(s, "impl AstNode for {name} {{").unwrap();
        writeln!(
            s,
            "    fn can_cast(kind: SyntaxKind) -> bool {{\n        matches!(\n            kind,"
        )
        .unwrap();
        let alts = members
            .iter()
            .map(|m| format!("SyntaxKind::{m}"))
            .collect::<Vec<_>>()
            .join("\n                | ");
        writeln!(s, "            {alts}\n        )\n    }}").unwrap();
        writeln!(s, "    fn cast(syntax: SyntaxNode) -> Option<Self> {{\n        Some(match syntax.kind() {{").unwrap();
        for m in members {
            writeln!(
                s,
                "            SyntaxKind::{m} => {name}::{m}({m} {{ syntax }}),"
            )
            .unwrap();
        }
        writeln!(s, "            _ => return None,\n        }})\n    }}").unwrap();
        writeln!(
            s,
            "    fn syntax(&self) -> &SyntaxNode {{\n        match self {{"
        )
        .unwrap();
        for m in members {
            writeln!(s, "            {name}::{m}(it) => &it.syntax,").unwrap();
        }
        writeln!(s, "        }}\n    }}\n}}\n").unwrap();
        for m in members {
            writeln!(
                s,
                "impl From<{m}> for {name} {{\n    fn from(node: {m}) -> {name} {{\n        {name}::{m}(node)\n    }}\n}}\n"
            )
            .unwrap();
        }
    }
    s
}

fn gen_rule(model: &Model<'_>, rule: &Rule) -> String {
    match rule {
        Rule::Labeled { rule, .. } => gen_rule(model, rule),
        Rule::Node(n) => {
            let name = &model.grammar[*n].name;
            match model.enums.get(name) {
                Some(members) => format!(
                    "R::Any(&[{}])",
                    members
                        .iter()
                        .map(|m| format!("K::{m}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                None => format!("R::Kind(K::{name})"),
            }
        },
        Rule::Token(t) => format!("R::Kind(K::{})", model.token_kind(&model.grammar[*t].name)),
        Rule::Seq(rs) => format!(
            "R::Seq(&[{}])",
            rs.iter()
                .map(|r| gen_rule(model, r))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Rule::Alt(rs) => format!(
            "R::Alt(&[{}])",
            rs.iter()
                .map(|r| gen_rule(model, r))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Rule::Opt(r) => format!("R::Opt(&{})", gen_rule(model, r)),
        Rule::Rep(r) => format!("R::Rep(&{})", gen_rule(model, r)),
    }
}

fn gen_grammar(model: &Model<'_>) -> String {
    let mut s = String::from(HEADER);
    s.push_str("use super::Rule as R;\nuse crate::SyntaxKind as K;\n\n/// Each node kind's rule over its non-trivia children.\npub(super) static RULES: &[(K, R)] = &[\n");
    for name in &model.node_order {
        writeln!(
            s,
            "    (K::{name}, {}),",
            gen_rule(model, model.nodes[name])
        )
        .unwrap();
    }
    s.push_str("];\n");
    s
}

fn check_or_update(path: &Path, generated: &str) -> bool {
    let current = std::fs::read_to_string(path).unwrap_or_default();
    if current == generated {
        return true;
    }
    if std::env::var_os("UPDATE_GENERATED").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, generated).unwrap();
        return true;
    }
    false
}

fn rustfmt(source: String) -> String {
    use std::io::Write;
    let mut child = std::process::Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("rustfmt");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "rustfmt failed on generated code");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn generated_code_is_fresh() {
    let kinds = read_kinds();
    let text = std::fs::read_to_string(root().join("kestrel.ungram")).unwrap();
    let grammar: Grammar = text.parse().expect("kestrel.ungram parses");
    let model = Model::new(&grammar, kinds.clone());

    // Every node rule names a node kind or a union; every node kind except
    // the special/legacy ones has a rule.
    let ruled: BTreeSet<_> = model.nodes.keys().cloned().collect();
    let unruled: Vec<_> = model
        .node_kinds
        .iter()
        .filter(|k| !ruled.contains(*k) && !UNRULED.contains(&k.as_str()))
        .collect();
    assert!(
        unruled.is_empty(),
        "node kinds without a rule in kestrel.ungram: {unruled:?}"
    );

    let files = [
        (
            root().join("src/generated/kinds.rs"),
            rustfmt(gen_kinds(&model.kinds)),
        ),
        (
            root().join("src/ast/generated.rs"),
            rustfmt(gen_nodes(&model)),
        ),
        (
            root().join("src/validate/generated.rs"),
            rustfmt(gen_grammar(&model)),
        ),
    ];
    let stale: Vec<_> = files
        .iter()
        .filter(|(path, text)| !check_or_update(path, text))
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        stale.is_empty(),
        "generated files are stale: {stale:?}\nrun `UPDATE_GENERATED=1 cargo test -p kestrel-syntax-tree --test sourcegen`"
    );
}

/// Node kinds that exist but are never produced by the parser.
const UNRULED: &[&str] = &[
    "Root",
    "DeclarationItem",
    "StringLiteralPart",
    "ErrorPattern",
    "AssociatedTypeBound",
];
