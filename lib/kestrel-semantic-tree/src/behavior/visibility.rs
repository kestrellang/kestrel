use std::fmt;
use std::sync::Arc;

use kestrel_span::Span;
use semantic_tree::{behavior::Behavior, symbol::Symbol};

use crate::{behavior::KestrelBehaviorKind, language::KestrelLanguage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Fileprivate,
    Internal,
    Public,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Private => write!(f, "private"),
            Visibility::Fileprivate => write!(f, "fileprivate"),
            Visibility::Internal => write!(f, "internal"),
            Visibility::Public => write!(f, "public"),
        }
    }
}

impl Visibility {
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "public" => Some(Visibility::Public),
            "private" => Some(Visibility::Private),
            "internal" => Some(Visibility::Internal),
            "fileprivate" => Some(Visibility::Fileprivate),
            _ => None,
        }
    }
}

pub fn find_visibility_scope(
    visibility: Option<&Visibility>,
    parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
    root: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Arc<dyn Symbol<KestrelLanguage>> {
    use crate::symbol::kind::KestrelSymbolKind;

    fn find_ancestor_of_kind(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        kind: KestrelSymbolKind,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let mut current = Some(symbol.clone());

        while let Some(s) = current {
            if s.metadata().kind() == kind {
                return Some(s);
            }
            current = s.metadata().parent();
        }

        None
    }

    match visibility {
        Some(Visibility::Private) => parent.cloned().unwrap_or_else(|| root.clone()),
        Some(Visibility::Fileprivate) => parent
            .and_then(|p| find_ancestor_of_kind(p, KestrelSymbolKind::SourceFile))
            .unwrap_or_else(|| root.clone()),
        Some(Visibility::Internal) | Some(Visibility::Public) | None => root.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct VisibilityBehavior {
    visibility: Option<Visibility>,
    #[allow(dead_code)]
    visibility_span: Span,
    visibility_scope: Arc<dyn Symbol<KestrelLanguage>>,
}

impl Behavior<KestrelLanguage> for VisibilityBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Visibility
    }
}

impl VisibilityBehavior {
    /// Create a new VisibilityBehavior
    pub fn new(
        visibility: Option<Visibility>,
        visibility_span: Span,
        visibility_scope: Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Self {
        VisibilityBehavior {
            visibility,
            visibility_span,
            visibility_scope,
        }
    }

    /// Get the visibility
    pub fn visibility(&self) -> Option<&Visibility> {
        self.visibility.as_ref()
    }

    /// Get the visibility scope
    pub fn visibility_scope(&self) -> &Arc<dyn Symbol<KestrelLanguage>> {
        &self.visibility_scope
    }
}
