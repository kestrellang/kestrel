//! Typed views over the CST.
//!
//! Every node kind in `kestrel.ungram` has a struct here wrapping its
//! `SyntaxNode`, with one accessor per child the grammar names; every union
//! (`Item`, `Expr`, `Ty`…) is an enum. The views are generated
//! (`generated.rs`) and hold no semantic information — they only name the
//! shape the parser builds. Hand-written conveniences that need more than
//! "the n-th child of kind K" live in `ext.rs`.
//!
//! Later stages keep an [`AstPtr`] — kind + text range, `Send`, cheap to
//! hash — instead of a `SyntaxNode` (which is `!Send` and pins the whole
//! tree), and resolve it against the parse when they need the syntax back.

mod ext;
mod generated;

use std::marker::PhantomData;

pub use ext::*;
pub use generated::*;

use crate::{SyntaxKind, SyntaxNode, SyntaxNodePtr, SyntaxToken};

/// A typed view of a CST node.
pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(syntax: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

/// The children of a node that cast to `N`, in order.
#[derive(Debug, Clone)]
pub struct AstChildren<N> {
    inner: rowan::SyntaxNodeChildren<crate::KestrelLanguage>,
    _ty: PhantomData<N>,
}

impl<N: AstNode> Iterator for AstChildren<N> {
    type Item = N;
    fn next(&mut self) -> Option<N> {
        self.inner.by_ref().find_map(N::cast)
    }
}

/// A stable, `Send` handle to a typed node: its kind and text range. Keep
/// this across stages; turn it back into the view with [`AstPtr::to_node`]
/// against the same file's tree.
#[derive(Debug)]
pub struct AstPtr<N> {
    raw: SyntaxNodePtr,
    _ty: PhantomData<fn() -> N>,
}

impl<N> Clone for AstPtr<N> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<N> Copy for AstPtr<N> {}
impl<N> PartialEq for AstPtr<N> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<N> Eq for AstPtr<N> {}
impl<N> std::hash::Hash for AstPtr<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state)
    }
}

impl<N: AstNode> AstPtr<N> {
    pub fn new(node: &N) -> Self {
        Self {
            raw: SyntaxNodePtr::new(node.syntax()),
            _ty: PhantomData,
        }
    }

    /// The node this points at, found in `root` (the same file's tree).
    pub fn to_node(&self, root: &SyntaxNode) -> Option<N> {
        N::cast(self.raw.try_to_node(root)?)
    }

    pub fn syntax_ptr(&self) -> SyntaxNodePtr {
        self.raw
    }

    pub fn text_range(&self) -> rowan::TextRange {
        self.raw.text_range()
    }

    pub fn kind(&self) -> SyntaxKind {
        self.raw.kind()
    }
}

/// Child lookup used by the generated accessors.
pub mod support {
    use super::{AstChildren, AstNode};
    use crate::{SyntaxKind, SyntaxNode, SyntaxToken};
    use std::marker::PhantomData;

    pub fn child<N: AstNode>(parent: &SyntaxNode) -> Option<N> {
        parent.children().find_map(N::cast)
    }

    pub fn children<N: AstNode>(parent: &SyntaxNode) -> AstChildren<N> {
        AstChildren {
            inner: parent.children(),
            _ty: PhantomData,
        }
    }

    pub fn token(parent: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
        nth_token(parent, kind, 0)
    }

    pub fn nth_token(parent: &SyntaxNode, kind: SyntaxKind, n: usize) -> Option<SyntaxToken> {
        parent
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == kind)
            .nth(n)
    }
}

/// The first non-trivia token directly inside `node`.
pub fn first_token(node: &SyntaxNode) -> Option<SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !t.kind().is_trivia())
}
