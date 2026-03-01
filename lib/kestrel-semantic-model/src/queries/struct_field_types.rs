//! StructFieldTypes query - get (name, span, type) for a struct's fields

use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::StructFields;
use crate::query::Query;

#[derive(Debug, Clone)]
pub struct StructFieldTypeInfo {
    pub name: String,
    pub span: Span,
    pub ty: Ty,
    pub is_computed: bool,
}

/// Get all field types (direct children) for a struct symbol.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StructFieldTypes {
    pub struct_id: SymbolId,
}

impl Query for StructFieldTypes {
    type Output = Vec<StructFieldTypeInfo>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        model
            .query(StructFields {
                struct_id: self.struct_id,
            })
            .into_iter()
            .map(|f| StructFieldTypeInfo {
                name: f.name,
                span: f.span,
                ty: f.ty,
                is_computed: f.is_computed,
            })
            .collect()
    }
}
