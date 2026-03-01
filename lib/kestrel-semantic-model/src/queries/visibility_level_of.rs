//! VisibilityLevelOf query - get the visibility level of a symbol

use kestrel_semantic_tree::behavior::visibility::{Visibility, VisibilityBehavior};
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Visibility level for comparison (higher = more visible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisibilityLevel {
    Private = 1,
    Fileprivate = 2,
    Internal = 3,
    Public = 4,
}

impl VisibilityLevel {
    pub fn from_visibility(vis: Option<&Visibility>) -> Self {
        match vis {
            Some(Visibility::Public) => VisibilityLevel::Public,
            Some(Visibility::Internal) => VisibilityLevel::Internal,
            Some(Visibility::Fileprivate) => VisibilityLevel::Fileprivate,
            Some(Visibility::Private) => VisibilityLevel::Private,
            None => VisibilityLevel::Internal, // Default is internal
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            VisibilityLevel::Public => "public",
            VisibilityLevel::Internal => "internal",
            VisibilityLevel::Fileprivate => "fileprivate",
            VisibilityLevel::Private => "private",
        }
    }
}

/// Get the visibility level of a symbol.
///
/// Extracts the `VisibilityBehavior` from the symbol and converts it to a
/// comparable `VisibilityLevel`. Defaults to `Internal` if no visibility is set.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VisibilityLevelOf {
    pub symbol_id: SymbolId,
}

impl Query for VisibilityLevelOf {
    type Output = VisibilityLevel;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor { id: self.symbol_id }) else {
            return VisibilityLevel::Internal;
        };

        let vis = symbol
            .metadata()
            .get_behavior::<VisibilityBehavior>()
            .and_then(|vb| vb.visibility().cloned());
        VisibilityLevel::from_visibility(vis.as_ref())
    }
}
