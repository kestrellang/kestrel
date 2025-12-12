use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use semantic_tree::symbol::Symbol;

/// Build-phase declaration builder.
///
/// This trait mirrors the build-specific surface of `kestrel_semantic_tree_builder::Resolver`.
pub trait Builder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>>;

    fn is_terminal(&self) -> bool {
        false
    }
}

impl<T: kestrel_semantic_tree_builder::Resolver + ?Sized> Builder for T {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        kestrel_semantic_tree_builder::Resolver::build_declaration(self, syntax, source, parent, root)
    }

    fn is_terminal(&self) -> bool {
        kestrel_semantic_tree_builder::Resolver::is_terminal(self)
    }
}

/// A by-value adapter that presents a `kestrel_semantic_tree_builder::Resolver` as a `Builder`.
pub struct BuilderRef<'a>(pub &'a dyn kestrel_semantic_tree_builder::Resolver);

impl Builder for BuilderRef<'_> {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        self.0.build_declaration(syntax, source, parent, root)
    }

    fn is_terminal(&self) -> bool {
        self.0.is_terminal()
    }
}

/// Registry mapping `SyntaxKind` to build-phase `Builder` implementations.
#[derive(Default)]
pub struct BuilderRegistry {
    inner: kestrel_semantic_tree_builder::ResolverRegistry,
}

impl BuilderRegistry {
    pub fn new() -> Self {
        Self {
            inner: kestrel_semantic_tree_builder::ResolverRegistry::new(),
        }
    }

    pub fn get(&self, kind: SyntaxKind) -> Option<BuilderRef<'_>> {
        self.inner.get(kind).map(BuilderRef)
    }
}
