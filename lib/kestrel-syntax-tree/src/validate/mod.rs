//! Conformance of a tree to `kestrel.ungram`.
//!
//! Each node kind's rule (generated into `generated.rs`) is matched against
//! the kinds of its non-trivia children. The parser's contract is that an
//! error-free parse conforms; the parser tests run [`validate`] over the
//! whole corpus, and debug builds of the parser check every error-free tree.

mod generated;

use crate::{SyntaxKind, SyntaxNode};

/// A rule over a node's children.
#[derive(Debug)]
pub enum Rule {
    /// Exactly this node or token kind.
    Kind(SyntaxKind),
    /// One node of any of these kinds (a union like `Expr`).
    Any(&'static [SyntaxKind]),
    Seq(&'static [Rule]),
    Alt(&'static [Rule]),
    Opt(&'static Rule),
    Rep(&'static Rule),
}

/// A node whose children do not match its rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub kind: SyntaxKind,
    pub range: rowan::TextRange,
    /// The node's non-trivia children, as found.
    pub children: Vec<SyntaxKind>,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}@{:?} does not match its rule; children: {:?}",
            self.kind, self.range, self.children
        )
    }
}

/// The rule for `kind`, if the grammar has one.
pub fn rule(kind: SyntaxKind) -> Option<&'static Rule> {
    generated::RULES
        .iter()
        .find_map(|(k, r)| (*k == kind).then_some(r))
}

/// Every node in `root` that does not conform. `Error` and `Missing` nodes
/// (only present when the parse reported errors) are skipped with their
/// contents, and so is any node that has one among its children.
pub fn validate(root: &SyntaxNode) -> Vec<Violation> {
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), SyntaxKind::Error | SyntaxKind::Missing) {
            continue;
        }
        let children: Vec<SyntaxKind> = node
            .children_with_tokens()
            .map(|e| e.kind())
            .filter(|k| !k.is_trivia())
            .collect();
        let recovered = children
            .iter()
            .any(|k| matches!(k, SyntaxKind::Error | SyntaxKind::Missing));
        let ok = recovered
            || rule(node.kind()).is_some_and(|r| ends(r, &children, 0).contains(&children.len()));
        if !ok {
            out.push(Violation {
                kind: node.kind(),
                range: node.text_range(),
                children,
            });
        }
        stack.extend(node.children());
    }
    out
}

/// All positions where `rule` can end when matched from `start`.
fn ends(rule: &Rule, kinds: &[SyntaxKind], start: usize) -> Vec<usize> {
    match rule {
        Rule::Kind(k) => one(kinds.get(start) == Some(k), start),
        Rule::Any(ks) => one(kinds.get(start).is_some_and(|k| ks.contains(k)), start),
        Rule::Seq(rules) => {
            let mut positions = vec![start];
            for r in *rules {
                let mut next = Vec::new();
                for p in positions {
                    for e in ends(r, kinds, p) {
                        if !next.contains(&e) {
                            next.push(e);
                        }
                    }
                }
                if next.is_empty() {
                    return next;
                }
                positions = next;
            }
            positions
        },
        Rule::Alt(rules) => {
            let mut out = Vec::new();
            for r in *rules {
                for e in ends(r, kinds, start) {
                    if !out.contains(&e) {
                        out.push(e);
                    }
                }
            }
            out
        },
        Rule::Opt(r) => {
            let mut out = vec![start];
            for e in ends(r, kinds, start) {
                if !out.contains(&e) {
                    out.push(e);
                }
            }
            out
        },
        Rule::Rep(r) => {
            let mut out = vec![start];
            let mut frontier = vec![start];
            while let Some(p) = frontier.pop() {
                for e in ends(r, kinds, p) {
                    if e > p && !out.contains(&e) {
                        out.push(e);
                        frontier.push(e);
                    }
                }
            }
            out
        },
    }
}

fn one(hit: bool, start: usize) -> Vec<usize> {
    if hit { vec![start + 1] } else { Vec::new() }
}
