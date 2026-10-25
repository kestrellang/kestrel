//! Kestrel Syntax Tree
//!
//! This crate defines the syntax tree representation for the Kestrel language
//! using the `rowan` library for a lossless, resilient syntax tree implementation.
//!
//! # Overview
//!
//! The syntax tree uses `rowan`, which provides:
//! - **Lossless**: Preserves all source text including whitespace and comments
//! - **Immutable**: Syntax trees are immutable and can be safely shared
//! - **Incremental**: Supports efficient incremental parsing
//!
//! # Example
//!
//! ```
//! use kestrel_syntax_tree::{GreenNodeBuilder, SyntaxKind, SyntaxNode};
//!
//! let mut builder = GreenNodeBuilder::new();
//! builder.start_node(SyntaxKind::ModulePath.into());
//! builder.token(SyntaxKind::Identifier.into(), "Main");
//! builder.finish_node();
//!
//! let green = builder.finish();
//! let syntax = SyntaxNode::new_root(green);
//!
//! assert_eq!(syntax.kind(), SyntaxKind::ModulePath);
//! ```

use rowan::Language;

// Re-export for use by parsers
pub use rowan::{GreenNode, GreenNodeBuilder};

mod generated;
pub use generated::kinds::SyntaxKind;

pub mod ast;
pub mod validate;

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl SyntaxKind {
    /// Whether this kind is trivia — present in the tree for fidelity, skipped
    /// by every grammar rule.
    ///
    /// The set is owned by [`Token::is_trivia`]; this is its image under
    /// `From<Token>`, and `trivia_agrees_with_the_lexer` proves the two stay in
    /// step in both directions. The duplication is unavoidable — the tree is
    /// built from `SyntaxKind`, not `Token` — but the *drift* is not.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::LineComment
                | SyntaxKind::BlockComment
        )
    }

    /// Whether this kind is a type node — something `ast_type_from_cst` can
    /// turn into an `AstType`.
    ///
    /// Excludes `TyList`, which *contains* types (a function parameter list)
    /// but is not one. `every_ty_kind_is_a_type_node` proves the set covers
    /// every `Ty*` variant except that one, so appending a type kind to the
    /// enum without listing it here fails the build rather than making the
    /// new syntax invisible to the AST builder.
    pub fn is_type(self) -> bool {
        matches!(
            self,
            SyntaxKind::Ty
                | SyntaxKind::TyPath
                | SyntaxKind::TyTuple
                | SyntaxKind::TyFunction
                | SyntaxKind::TyArray
                | SyntaxKind::TyDictionary
                | SyntaxKind::TyOptional
                | SyntaxKind::TyResult
                | SyntaxKind::TyUnit
                | SyntaxKind::TyNever
                | SyntaxKind::TyInferred
                | SyntaxKind::TySome
                | SyntaxKind::TyRef
                | SyntaxKind::TyMutRef
                | SyntaxKind::TyParen
        )
    }

    /// `Ty*` kinds that are deliberately **not** type nodes. Each needs a
    /// reason — this list is the only way past `every_ty_kind_is_a_type_node`.
    #[cfg(test)]
    const NON_TYPE_TY_KINDS: &'static [(SyntaxKind, &'static str)] = &[(
        SyntaxKind::TyList,
        "a list of parameter types, not a type itself",
    )];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KestrelLanguage;

impl Language for KestrelLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // `kind_to_raw` is `kind as u16`, so the inverse is a plain index into
        // the declaration-order table. This used to be 258 hand-written
        // `const NAME: u16 = SyntaxKind::Name as u16;` declarations plus 258
        // hand-written match arms over `raw.0` — and because the scrutinee was
        // a `u16`, rustc could not check either list. A kind appended to the
        // enum without a matching arm silently read back as `Error`, which is
        // the *recovery* marker, so the tree would look damaged rather than
        // unknown (F27). `syntax_kind_table_round_trips` proves `ALL` is
        // complete and in order.
        SyntaxKind::ALL
            .get(raw.0 as usize)
            .copied()
            .unwrap_or(SyntaxKind::Error)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<KestrelLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<KestrelLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<KestrelLanguage>;
pub type SyntaxNodePtr = rowan::ast::SyntaxNodePtr<KestrelLanguage>;

pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::Token;

    #[test]
    fn test_syntax_kind_conversion() {
        // Test that Token to SyntaxKind conversion works
        assert_eq!(
            SyntaxKind::from(kestrel_lexer::Token::Module),
            SyntaxKind::Module
        );
        assert_eq!(
            SyntaxKind::from(kestrel_lexer::Token::Identifier),
            SyntaxKind::Identifier
        );
        assert_eq!(SyntaxKind::from(kestrel_lexer::Token::Dot), SyntaxKind::Dot);
    }

    /// `SyntaxKind::ALL` is the hand-written inverse of `kind as u16`. Three
    /// properties make it safe to hand-write; this test is all three.
    ///
    /// 1. **Ordered** — `ALL[n] as u16 == n`, so indexing by a raw value is the
    ///    correct inverse.
    /// 2. **Complete** — every kind round-trips through rowan's raw form. A
    ///    kind appended to the enum but not to `ALL` fails here instead of
    ///    silently reading back as `Error`, which is the *recovery* marker: the
    ///    tree would look damaged rather than unknown (F27).
    /// 3. **Total** — the entry for `Error` itself round-trips, so the
    ///    out-of-range fallback is not masking a real kind.
    #[test]
    fn syntax_kind_table_round_trips() {
        for (index, &kind) in SyntaxKind::ALL.iter().enumerate() {
            assert_eq!(
                kind as usize, index,
                "SyntaxKind::ALL[{index}] is {kind:?}, whose discriminant is {}. \
                 The table must be in declaration order — a kind was inserted \
                 mid-list instead of appended.",
                kind as usize
            );
            let raw = KestrelLanguage::kind_to_raw(kind);
            assert_eq!(
                KestrelLanguage::kind_from_raw(raw),
                kind,
                "{kind:?} does not round-trip through rowan's raw form"
            );
        }
        // Completeness: `__NotAKind` sits immediately after the last real
        // variant, so its discriminant IS the count. Without this, a table
        // missing its final entries still round-trips — every entry it *does*
        // hold is correct, and the missing kinds are simply never tested.
        assert_eq!(
            SyntaxKind::__NotAKind as usize,
            SyntaxKind::ALL.len(),
            "SyntaxKind::ALL is missing {} kind(s) — append the new variant(s) \
             to the table too",
            SyntaxKind::__NotAKind as usize - SyntaxKind::ALL.len()
        );

        // Anything past the table is genuinely unknown and must read as Error.
        let past_end = rowan::SyntaxKind(SyntaxKind::ALL.len() as u16);
        assert_eq!(KestrelLanguage::kind_from_raw(past_end), SyntaxKind::Error);
    }

    /// The type-node set is derivable from the enum itself: a `Ty*` variant is
    /// a type node unless it is explicitly excused. This is the check that was
    /// missing when `is_type_node` (14 variants) and `is_type_kind` (12) drifted
    /// apart — `TyRef`/`TyMutRef` were appended to only one of them, and the
    /// omission was masked only by the parser wrapping `TyRef` inside a `Ty`.
    #[test]
    fn every_ty_kind_is_a_type_node() {
        for &kind in SyntaxKind::ALL {
            let name = format!("{kind:?}");
            // `Type*` (TypeBound, TypeParameter, …) are not type *nodes*.
            if !name.starts_with("Ty") || name.starts_with("Type") {
                assert!(
                    !kind.is_type(),
                    "{kind:?} is marked a type node but is not a `Ty*` kind"
                );
                continue;
            }
            let excused = SyntaxKind::NON_TYPE_TY_KINDS
                .iter()
                .find(|(k, _)| *k == kind);
            match excused {
                Some((_, reason)) => assert!(
                    !kind.is_type(),
                    "{kind:?} is excused from being a type node ({reason}) \
                     but `is_type` claims it is one"
                ),
                None => assert!(
                    kind.is_type(),
                    "{kind:?} is a `Ty*` kind but `SyntaxKind::is_type` does not \
                     list it. Add it, or add it to NON_TYPE_TY_KINDS with a reason."
                ),
            }
        }
    }

    /// The trivia set is defined once, on `Token`. `SyntaxKind::is_trivia` is
    /// its image under `From<Token>`, and the two must not drift: a token the
    /// grammar skips whose kind is not marked trivia breaks CST navigation,
    /// and a kind marked trivia with no trivia token behind it can never match.
    #[test]
    fn trivia_agrees_with_the_lexer() {
        // Forward: every trivia token's kind is trivia, and no other token's is.
        let trivia_tokens = [
            Token::Whitespace,
            Token::Newline,
            Token::LineComment,
            Token::BlockComment,
        ];
        for token in &trivia_tokens {
            assert!(token.is_trivia(), "{token:?} must be trivia");
            let kind = SyntaxKind::from(token.clone());
            assert!(
                kind.is_trivia(),
                "{token:?} is trivia but SyntaxKind::{kind:?} is not"
            );
        }
        for token in [Token::Identifier, Token::Func, Token::LBrace, Token::String] {
            assert!(!token.is_trivia());
            assert!(!SyntaxKind::from(token).is_trivia());
        }

        // Backward, and the half that actually catches drift: no kind may be
        // marked trivia without a trivia token behind it. Splitting `///` out
        // of `LineComment` adds a fifth trivia kind and fails here, which is
        // the signal to add the matching `Token` arm rather than teach one
        // call site about the new kind.
        let trivia_kinds: Vec<_> = SyntaxKind::ALL
            .iter()
            .copied()
            .filter(|k| k.is_trivia())
            .collect();
        let expected: Vec<_> = trivia_tokens
            .iter()
            .cloned()
            .map(SyntaxKind::from)
            .collect();
        assert_eq!(
            trivia_kinds, expected,
            "SyntaxKind's trivia set drifted from Token's — the set is owned by \
             Token::is_trivia; update it there and map the new token"
        );

        // `is_inline_trivia` is a strict subset differing only in `Newline`.
        for token in &trivia_tokens {
            assert_eq!(
                token.is_inline_trivia(),
                token.is_trivia() && *token != Token::Newline
            );
        }
    }

    #[test]
    fn test_basic_tree() {
        // Test building a simple syntax tree
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::Root.into());
        builder.token(SyntaxKind::Identifier.into(), "test");
        builder.finish_node();

        let green = builder.finish();
        let root = SyntaxNode::new_root(green);

        assert_eq!(root.kind(), SyntaxKind::Root);
    }
}
