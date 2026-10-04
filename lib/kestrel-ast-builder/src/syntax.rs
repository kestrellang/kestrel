//! Resolving the syntax handles that components keep.
//!
//! Components never hold a `SyntaxNode` (it is `!Send` and pins the whole
//! tree): a declaration keeps a [`CstNode`] / [`Valued`] pointer, and the file
//! entity keeps its green tree in [`FileSyntax`]. These helpers turn a
//! pointer back into a node, through either the mutable `World` or a
//! dependency-tracking `QueryContext`.

use kestrel_hecs::component::Component;
use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_syntax_tree::{SyntaxNode, SyntaxNodePtr};

use crate::components::{CstNode, FileId, FileSyntax, Valued};

/// Read access to components, shared by `World` and `QueryContext`.
pub trait Components {
    fn component<T: Component>(&self, entity: Entity) -> Option<&T>;
}

impl Components for World {
    fn component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get::<T>(entity)
    }
}

impl Components for QueryContext<'_> {
    fn component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get::<T>(entity)
    }
}

/// The `SourceFile` node of the file `entity` was declared in (or of
/// `entity` itself, when it is a file).
pub fn file_root(c: &impl Components, entity: Entity) -> Option<SyntaxNode> {
    let file = c.component::<FileId>(entity).map_or(entity, |f| f.0);
    Some(c.component::<FileSyntax>(file)?.root())
}

/// Resolve a pointer into the file `entity` was declared in.
pub fn resolve(c: &impl Components, entity: Entity, ptr: &SyntaxNodePtr) -> Option<SyntaxNode> {
    ptr.try_to_node(&file_root(c, entity)?)
}

/// The declaration node of `entity` (from its [`CstNode`]).
pub fn cst_node(c: &impl Components, entity: Entity) -> Option<SyntaxNode> {
    resolve(c, entity, &c.component::<CstNode>(entity)?.0)
}

/// The body node of `entity` (from its [`Valued`]).
pub fn valued_node(c: &impl Components, entity: Entity) -> Option<SyntaxNode> {
    resolve(c, entity, &c.component::<Valued>(entity)?.0)
}
