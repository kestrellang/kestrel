//! Debug and printing utilities for semantic models.

use std::sync::Arc;

use kestrel_prelude::primitives;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::behavior::member_access::MemberAccessBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::behavior::visibility::VisibilityBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::Symbol;

/// Print the semantic model (shows symbol hierarchy)
pub fn print_semantic_model(model: &SemanticModel) {
    let root = model.root();
    let children = root.metadata().children();

    println!("{} top-level symbols\n", children.len());

    for child in children {
        print_symbol(&child, 0);
    }
}

/// Print symbols from a semantic model
pub fn print_model_symbols(model: &SemanticModel) {
    // Walk the tree and collect all symbols
    fn collect_symbols(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        symbols: &mut Vec<(String, String)>,
    ) {
        let name = symbol.metadata().name().value.clone();
        let kind = format!("{:?}", symbol.metadata().kind());
        symbols.push((name, kind));

        for child in symbol.metadata().children() {
            collect_symbols(&child, symbols);
        }
    }

    let mut symbols = Vec::new();
    for child in model.root().metadata().children() {
        collect_symbols(&child, &mut symbols);
    }

    println!("Symbols:");
    println!("  {} symbols\n", symbols.len());

    symbols.sort_by(|a, b| a.0.cmp(&b.0));

    println!("  {:<30} {:<15}", "Name", "Kind");
    println!("  {}", "-".repeat(45));

    for (name, kind) in symbols {
        println!("  {:<30} {:<15}", name, kind);
    }
}

/// Format a type for display
pub fn format_type(ty: &Ty) -> String {
    match ty.kind() {
        TyKind::Unit => "()".to_string(),
        TyKind::Never => "!".to_string(),
        TyKind::Int(bits) => format!("{:?}", bits),
        TyKind::Float(bits) => format!("{:?}", bits),
        TyKind::Bool => primitives::BOOL.to_string(),
        TyKind::String => primitives::STRING.to_string(),
        TyKind::Tuple(elements) => {
            let elem_strs: Vec<String> = elements.iter().map(format_type).collect();
            format!("({})", elem_strs.join(", "))
        }
        TyKind::Array(element_type) => {
            format!("[{}]", format_type(element_type))
        }
        TyKind::Function {
            params,
            return_type,
        } => {
            let param_strs: Vec<String> = params.iter().map(format_type).collect();
            format!(
                "({}) -> {}",
                param_strs.join(", "),
                format_type(return_type)
            )
        }
        TyKind::TypeParameter(param_symbol) => param_symbol.metadata().name().value.clone(),
        TyKind::Protocol {
            symbol: protocol_symbol,
            substitutions,
        } => {
            let name = protocol_symbol.metadata().name().value.clone();
            if substitutions.is_empty() {
                name
            } else {
                let args: Vec<String> = substitutions
                    .iter()
                    .map(|(_, ty)| format_type(ty))
                    .collect();
                format!("{}[{}]", name, args.join(", "))
            }
        }
        TyKind::Struct {
            symbol: struct_symbol,
            substitutions,
        } => {
            let name = struct_symbol.metadata().name().value.clone();
            if substitutions.is_empty() {
                name
            } else {
                let args: Vec<String> = substitutions
                    .iter()
                    .map(|(_, ty)| format_type(ty))
                    .collect();
                format!("{}[{}]", name, args.join(", "))
            }
        }
        TyKind::TypeAlias {
            symbol: type_alias_symbol,
            substitutions,
        } => {
            let name = type_alias_symbol.metadata().name().value.clone();
            if substitutions.is_empty() {
                name
            } else {
                let args: Vec<String> = substitutions
                    .iter()
                    .map(|(_, ty)| format_type(ty))
                    .collect();
                format!("{}[{}]", name, args.join(", "))
            }
        }
        TyKind::Error => "<error>".to_string(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::TypeVar(_) => "_".to_string(),
        TyKind::AssociatedType { symbol, container } => {
            let name = symbol.metadata().name().value.clone();
            match container {
                Some(container_ty) => format!("{}.{}", format_type(container_ty), name),
                None => name,
            }
        }
    }
}

/// Debug print a symbol and its children
pub fn print_symbol(symbol: &Arc<dyn Symbol<KestrelLanguage>>, level: usize) {
    let indent = "  ".repeat(level);
    let metadata = symbol.metadata();

    let behaviors = metadata.behaviors();
    let behaviors_str = if !behaviors.is_empty() {
        let behavior_strings: Vec<String> = behaviors
            .iter()
            .map(|b| format_behavior(b.as_ref()))
            .collect();
        format!(" [{}]", behavior_strings.join(", "))
    } else {
        String::new()
    };

    println!(
        "{}{:?} '{}'{}",
        indent,
        metadata.kind(),
        metadata.name().value,
        behaviors_str
    );

    for child in metadata.children() {
        print_symbol(&child, level + 1);
    }
}

/// Format a behavior for display
fn format_behavior(b: &dyn Behavior<KestrelLanguage>) -> String {
    // Try each behavior type
    if let Some(vb) = b.downcast_ref::<VisibilityBehavior>() {
        if let Some(vis) = vb.visibility() {
            return format!("Visibility({})", vis);
        }
        return "Visibility".to_string();
    }

    if let Some(tb) = b.downcast_ref::<TypedBehavior>() {
        return format!("Typed({})", format_type(tb.ty()));
    }

    if let Some(import_data) = b.downcast_ref::<ImportDataBehavior>() {
        let path = import_data.module_path().join(".");
        let items = import_data.items();
        if items.is_empty() {
            if let Some(alias) = import_data.alias() {
                return format!("Import({} as {})", path, alias);
            }
            return format!("Import({})", path);
        }
        let item_strs: Vec<String> = items
            .iter()
            .map(|i| {
                if let Some(alias) = &i.alias {
                    format!("{} as {}", i.name, alias)
                } else {
                    i.name.clone()
                }
            })
            .collect();
        return format!("Import({}.({}))", path, item_strs.join(", "));
    }

    if let Some(callable) = b.downcast_ref::<CallableBehavior>() {
        let params: Vec<String> = callable
            .parameters()
            .iter()
            .map(|p| {
                let label = p.external_label().unwrap_or("_");
                format!("{}: {}", label, format_type(&p.ty))
            })
            .collect();
        let ret = format_type(callable.return_type());
        return format!("Callable(({}) -> {})", params.join(", "), ret);
    }

    if let Some(fd) = b.downcast_ref::<FunctionDataBehavior>() {
        return format!(
            "FunctionData(has_body={}, is_static={})",
            fd.has_body(),
            fd.is_static()
        );
    }

    if let Some(vb) = b.downcast_ref::<ValueBehavior>() {
        return format!("Valued({})", format_type(vb.ty()));
    }

    if let Some(cb) = b.downcast_ref::<ConformancesBehavior>() {
        let conformances: Vec<String> = cb.conformances().iter().map(|t| format_type(t)).collect();
        return format!("Conformances({})", conformances.join(", "));
    }

    if let Some(eb) = b.downcast_ref::<ExecutableBehavior>() {
        let stmt_count = eb.body().statements.len();
        let has_yield = eb.body().yield_expr().is_some();
        return format!("Executable(stmts={}, has_yield={})", stmt_count, has_yield);
    }

    if let Some(ma) = b.downcast_ref::<MemberAccessBehavior>() {
        return format!("MemberAccess({})", ma.member_name());
    }

    "Unknown".to_string()
}
