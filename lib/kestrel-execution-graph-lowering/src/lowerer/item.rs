//! Item dispatch - routes symbols to their appropriate lowerers.

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::error::LoweringError;

use super::{lower_enum, lower_function, lower_struct};

/// Lower a symbol to MIR.
///
/// This is the main dispatch function that routes different symbol kinds
/// to their specific lowering implementations.
pub fn lower_item(ctx: &mut LoweringContext, symbol: &Arc<dyn Symbol<KestrelLanguage>>) {
    let kind = symbol.metadata().kind();
    let span = symbol.metadata().span().clone();

    match kind {
        KestrelSymbolKind::Function => {
            if let Ok(func_symbol) = symbol.clone().downcast_arc::<FunctionSymbol>() {
                lower_function(ctx, &func_symbol);
            }
        }

        KestrelSymbolKind::Initializer => {
            if let Ok(init_symbol) = symbol.clone().downcast_arc::<InitializerSymbol>() {
                lower_initializer(ctx, &init_symbol);
            }
        }

        KestrelSymbolKind::Struct => {
            if let Ok(struct_symbol) = symbol.clone().downcast_arc::<StructSymbol>() {
                lower_struct(ctx, &struct_symbol);

                // Also lower methods and initializers within the struct
                for child in symbol.metadata().children() {
                    let child_kind = child.metadata().kind();
                    if child_kind == KestrelSymbolKind::Function
                        || child_kind == KestrelSymbolKind::Initializer
                    {
                        lower_item(ctx, &child);
                    }
                }
            }
        }

        KestrelSymbolKind::Module => {
            // Recursively lower all children
            for child in symbol.metadata().children() {
                lower_item(ctx, &child);
            }
        }

        KestrelSymbolKind::SourceFile => {
            // Recursively lower all children
            for child in symbol.metadata().children() {
                lower_item(ctx, &child);
            }
        }

        KestrelSymbolKind::Enum => {
            if let Ok(enum_symbol) = symbol.clone().downcast_arc::<EnumSymbol>() {
                lower_enum(ctx, &enum_symbol);

                // Also lower methods within the enum
                for child in symbol.metadata().children() {
                    let child_kind = child.metadata().kind();
                    if child_kind == KestrelSymbolKind::Function {
                        lower_item(ctx, &child);
                    }
                }
            }
        }

        KestrelSymbolKind::Protocol => {
            // TODO: Lower protocol definitions
            ctx.emit_error(LoweringError::unsupported_item("Protocol", span));
        }

        KestrelSymbolKind::Extension => {
            // Extensions don't have their own MIR representation - they just add methods
            // to existing types. The methods are lowered as top-level functions with
            // qualified names based on the target type (e.g., Int.double for an extension
            // method on Int).
            //
            // Lower all methods and initializers within the extension
            for child in symbol.metadata().children() {
                let child_kind = child.metadata().kind();
                if child_kind == KestrelSymbolKind::Function
                    || child_kind == KestrelSymbolKind::Initializer
                {
                    lower_item(ctx, &child);
                }
            }
        }

        KestrelSymbolKind::TypeAlias => {
            // Type aliases don't generate MIR - they're expanded during type lowering
        }

        KestrelSymbolKind::EnumCase => {
            // Enum cases are handled as part of enum lowering
        }

        KestrelSymbolKind::Field => {
            // Fields are handled as part of struct lowering
        }

        KestrelSymbolKind::Import => {
            // Imports don't generate MIR
        }

        KestrelSymbolKind::TypeParameter => {
            // Type parameters are handled during type lowering
        }

        KestrelSymbolKind::AssociatedType => {
            // Associated types are handled during protocol lowering
        }
    }
}

/// Lower an initializer to MIR.
///
/// Initializers are lowered as regular functions with the signature:
/// `func Type.init(self: &var Type, args...) -> ()`
fn lower_initializer(ctx: &mut LoweringContext, init_symbol: &Arc<InitializerSymbol>) {
    // Initializers are very similar to functions, delegate to function lowering
    // with special handling for the implicit self parameter
    super::function::lower_initializer(ctx, init_symbol);
}
