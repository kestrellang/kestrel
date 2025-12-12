//! StructFields query - get field info for a struct symbol

use std::sync::Arc;

use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::Ty;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

#[derive(Debug, Clone)]
pub struct StructFieldInfo {
    pub field_id: SymbolId,
    pub name: String,
    pub span: Span,
    pub is_mutable: bool,
    pub ty: Ty,
}

/// Get all fields (direct children) of a struct symbol.
pub struct StructFields {
    pub struct_id: SymbolId,
}

impl Query for StructFields {
    type Output = Vec<StructFieldInfo>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = match model.query(SymbolFor { id: self.struct_id }) {
            Some(s) => s,
            None => return Vec::new(),
        };
        let struct_sym: Arc<StructSymbol> = match symbol.downcast_arc() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let struct_dyn: Arc<dyn Symbol<KestrelLanguage>> = struct_sym;
        struct_dyn
            .metadata()
            .children()
            .into_iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::Field)
            .filter_map(|child| {
                let field: Arc<FieldSymbol> = child.clone().downcast_arc().ok()?;
                let ty = child
                    .metadata()
                    .get_behavior::<TypedBehavior>()
                    .map(|typed| typed.ty().clone())
                    .unwrap_or_else(|| field.field_type().clone());
                Some(StructFieldInfo {
                    field_id: field.metadata().id(),
                    name: field.metadata().name().value.clone(),
                    span: field.metadata().span().clone(),
                    is_mutable: field.is_mutable(),
                    ty,
                })
            })
            .collect()
    }
}
