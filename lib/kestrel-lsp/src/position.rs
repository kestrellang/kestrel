//! Position-based lookup utilities for LSP features.
//!
//! Provides functions to find symbols and type information at a given source position.

use kestrel_compiler::Compilation;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::valued::ValueBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

/// Information about a symbol at a position.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// The symbol's name
    pub name: String,
    /// The symbol's kind (e.g., "function", "struct", "variable")
    pub kind: String,
    /// Type signature or description
    pub signature: String,
    /// Definition location (file_id, start, end)
    pub definition: Option<(usize, usize, usize)>,
}

/// Find the symbol at a given byte offset in a file.
///
/// Returns information about the symbol if found.
pub fn find_symbol_at_position(
    compilation: &Compilation,
    file_id: usize,
    offset: usize,
) -> Option<SymbolInfo> {
    let model = compilation.semantic_model()?;

    // Walk all symbols looking for one whose name span contains the offset
    fn find_in_symbol(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        offset: usize,
    ) -> Option<SymbolInfo> {
        let metadata = symbol.metadata();
        let name_span = &metadata.name().span;

        // Check if the cursor is on this symbol's name
        if name_span.file_id == file_id
            && offset >= name_span.start
            && offset <= name_span.end
        {
            let kind = format!("{:?}", metadata.kind());
            let signature = build_signature(symbol);
            let decl_span = metadata.declaration_span();

            return Some(SymbolInfo {
                name: metadata.name().value.clone(),
                kind,
                signature,
                definition: Some((decl_span.file_id, decl_span.start, decl_span.end)),
            });
        }

        // Check children
        for child in metadata.children() {
            if let Some(info) = find_in_symbol(&child, file_id, offset) {
                return Some(info);
            }
        }

        None
    }

    // Search from root
    let root = model.root();
    for child in root.metadata().children() {
        if let Some(info) = find_in_symbol(&child, file_id, offset) {
            return Some(info);
        }
    }

    None
}

/// Build a signature string for a symbol.
fn build_signature(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> String {
    let metadata = symbol.metadata();
    let kind = format!("{:?}", metadata.kind()).to_lowercase();
    let name = &metadata.name().value;

    // Check for callable behavior (functions, methods)
    if let Some(callable) = metadata.get_behavior::<CallableBehavior>() {
        let params: Vec<String> = callable
            .parameters()
            .iter()
            .map(|p| {
                let label = p.external_label().unwrap_or("_");
                if label == "_" {
                    format!("{}", p.ty)
                } else {
                    format!("{}: {}", label, p.ty)
                }
            })
            .collect();
        return format!(
            "func {}({}) -> {}",
            name,
            params.join(", "),
            callable.return_type()
        );
    }

    // Check for typed behavior (fields, constants)
    if let Some(typed) = metadata.get_behavior::<TypedBehavior>() {
        return format!("{} {}: {}", kind, name, typed.ty());
    }

    // Check for valued behavior (variables)
    if let Some(valued) = metadata.get_behavior::<ValueBehavior>() {
        return format!("{} {}: {}", kind, name, valued.ty());
    }

    // Default: just kind and name
    format!("{} {}", kind, name)
}

/// Information about a function call site for signature help.
#[derive(Debug, Clone)]
pub struct CallSiteInfo {
    /// The function name being called
    pub function_name: String,
    /// The function's parameters
    pub parameters: Vec<ParameterInfo>,
    /// The return type
    pub return_type: String,
    /// Which parameter the cursor is on (0-indexed)
    pub active_parameter: usize,
}

/// Information about a function parameter.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// The parameter label (e.g., "x: Int")
    pub label: String,
    /// Optional documentation
    pub documentation: Option<String>,
}

/// Find the function being called at a given position (for signature help).
///
/// This looks backwards from the cursor to find the opening `(` and then
/// identifies the function name, looking it up in the semantic model.
pub fn find_call_site_at_position(
    compilation: &Compilation,
    source: &str,
    _file_id: usize,
    offset: usize,
) -> Option<CallSiteInfo> {
    // Find the opening paren and function name by scanning backwards
    let before_cursor = &source[..offset.min(source.len())];

    // Count parens to handle nested calls and find the active parameter
    let mut paren_depth = 0;
    let mut comma_count = 0;
    let mut open_paren_pos = None;

    for (i, ch) in before_cursor.char_indices().rev() {
        match ch {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    open_paren_pos = Some(i);
                    break;
                }
                paren_depth -= 1;
            }
            ',' if paren_depth == 0 => comma_count += 1,
            _ => {}
        }
    }

    let open_paren_pos = open_paren_pos?;

    // Extract the function name before the `(`
    let before_paren = &source[..open_paren_pos];
    let function_name = extract_identifier_before(before_paren)?;

    // Look up the function in the semantic model
    let model = compilation.semantic_model()?;

    // Search for a function with this name
    fn find_function_by_name(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        name: &str,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let metadata = symbol.metadata();

        // Check if this symbol is a function with the matching name
        if metadata.name().value == name {
            if metadata.get_behavior::<CallableBehavior>().is_some() {
                return Some(symbol.clone());
            }
        }

        // Check children
        for child in metadata.children() {
            if let Some(found) = find_function_by_name(&child, name) {
                return Some(found);
            }
        }

        None
    }

    let root = model.root();
    let mut function_symbol = None;
    for child in root.metadata().children() {
        if let Some(found) = find_function_by_name(&child, &function_name) {
            function_symbol = Some(found);
            break;
        }
    }

    let function_symbol = function_symbol?;
    let callable = function_symbol.metadata().get_behavior::<CallableBehavior>()?;

    // Build parameter info
    let parameters: Vec<ParameterInfo> = callable
        .parameters()
        .iter()
        .map(|p| {
            let label = if let Some(ext) = p.external_label() {
                if ext == "_" {
                    format!("{}: {}", p.bind_name.value, p.ty)
                } else {
                    format!("{}: {}", ext, p.ty)
                }
            } else {
                format!("{}: {}", p.bind_name.value, p.ty)
            };
            ParameterInfo {
                label,
                documentation: None,
            }
        })
        .collect();

    Some(CallSiteInfo {
        function_name,
        parameters,
        return_type: callable.return_type().to_string(),
        active_parameter: comma_count,
    })
}

/// Extract an identifier from the end of a string.
fn extract_identifier_before(s: &str) -> Option<String> {
    let trimmed = s.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    // Find the start of the identifier (scan backwards for non-identifier chars)
    let mut start = trimmed.len();
    for (i, ch) in trimmed.char_indices().rev() {
        if ch.is_alphanumeric() || ch == '_' {
            start = i;
        } else {
            break;
        }
    }

    let ident = &trimmed[start..];
    if ident.is_empty() || ident.chars().next()?.is_numeric() {
        return None;
    }

    Some(ident.to_string())
}

/// Find all symbol definitions in a file and return their positions.
///
/// This is useful for document symbols.
pub fn find_all_symbols_in_file(
    compilation: &Compilation,
    file_id: usize,
) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    let Some(model) = compilation.semantic_model() else {
        return symbols;
    };

    fn collect_symbols(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        symbols: &mut Vec<SymbolInfo>,
    ) {
        let metadata = symbol.metadata();
        let span = metadata.span();

        // Only include symbols defined in this file
        if span.file_id == file_id && !span.is_synthetic() {
            let kind = format!("{:?}", metadata.kind());
            let signature = build_signature(symbol);
            let decl_span = metadata.declaration_span();

            symbols.push(SymbolInfo {
                name: metadata.name().value.clone(),
                kind,
                signature,
                definition: Some((decl_span.file_id, decl_span.start, decl_span.end)),
            });
        }

        // Collect children
        for child in metadata.children() {
            collect_symbols(&child, file_id, symbols);
        }
    }

    let root = model.root();
    for child in root.metadata().children() {
        collect_symbols(&child, file_id, &mut symbols);
    }

    symbols
}

/// A completion item for LSP.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The label to display
    pub label: String,
    /// The kind of completion (field, method, etc.)
    pub kind: CompletionKind,
    /// Detail/signature information
    pub detail: Option<String>,
    /// Text to insert (if different from label)
    pub insert_text: Option<String>,
}

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Field,
    Method,
    Function,
    Property,
}

/// Find completions for dot completion (e.g., `foo.` shows members of foo's type).
pub fn find_dot_completions(
    compilation: &Compilation,
    source: &str,
    file_id: usize,
    offset: usize,
) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    let model = match compilation.semantic_model() {
        Some(m) => m,
        None => return completions,
    };

    // Find the identifier before the dot
    let before_cursor = &source[..offset.min(source.len())];
    let trimmed = before_cursor.trim_end_matches('.');
    let var_name = match extract_identifier_before(trimmed) {
        Some(name) => name,
        None => return completions,
    };

    // Find the variable in the semantic model to get its type
    // We need to search for a local variable or parameter with this name
    let var_type = find_variable_type(compilation, file_id, offset, &var_name);

    let Some(ty) = var_type else {
        return completions;
    };

    // Based on the type, find available members
    match ty.kind() {
        TyKind::Struct { symbol, .. } => {
            // Add fields
            for child in symbol.metadata().children() {
                let kind = child.metadata().kind();
                if kind == KestrelSymbolKind::Field {
                    let name = child.metadata().name().value.clone();
                    let detail = child
                        .metadata()
                        .get_behavior::<TypedBehavior>()
                        .map(|t| t.ty().to_string());
                    completions.push(CompletionItem {
                        label: name,
                        kind: CompletionKind::Field,
                        detail,
                        insert_text: None,
                    });
                } else if kind == KestrelSymbolKind::Function {
                    let name = child.metadata().name().value.clone();
                    let detail = child
                        .metadata()
                        .get_behavior::<CallableBehavior>()
                        .map(|c| format!("({}) -> {}",
                            c.parameters().iter()
                                .map(|p| p.ty.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                            c.return_type()
                        ));
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Method,
                        detail,
                        insert_text: Some(format!("{}()", name)),
                    });
                }
            }

            // Add extension methods
            let extensions = model.extension_registry().get_extensions_for(symbol.metadata().id());
            for ext in extensions {
                for child in ext.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::Function {
                        let name = child.metadata().name().value.clone();
                        let detail = child
                            .metadata()
                            .get_behavior::<CallableBehavior>()
                            .map(|c| format!("({}) -> {}",
                                c.parameters().iter()
                                    .map(|p| p.ty.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                c.return_type()
                            ));
                        completions.push(CompletionItem {
                            label: name.clone(),
                            kind: CompletionKind::Method,
                            detail,
                            insert_text: Some(format!("{}()", name)),
                        });
                    }
                }
            }
        }
        TyKind::Enum { symbol, .. } => {
            // Add enum methods
            for child in symbol.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::Function {
                    let name = child.metadata().name().value.clone();
                    let detail = child
                        .metadata()
                        .get_behavior::<CallableBehavior>()
                        .map(|c| format!("({}) -> {}",
                            c.parameters().iter()
                                .map(|p| p.ty.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                            c.return_type()
                        ));
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Method,
                        detail,
                        insert_text: Some(format!("{}()", name)),
                    });
                }
            }
        }
        TyKind::String => {
            // String is a builtin - we'd need to look up String extension methods
            // For now, just indicate it's a String
        }
        _ => {}
    }

    completions
}

/// Find the type of a variable at a given position.
fn find_variable_type(
    compilation: &Compilation,
    file_id: usize,
    offset: usize,
    var_name: &str,
) -> Option<kestrel_semantic_tree::ty::Ty> {
    let model = compilation.semantic_model()?;

    // Walk the symbol tree to find a symbol matching this name
    // that we're "inside" based on position
    fn find_in_symbol(
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        file_id: usize,
        offset: usize,
        var_name: &str,
    ) -> Option<kestrel_semantic_tree::ty::Ty> {
        let metadata = symbol.metadata();
        let span = metadata.span();

        // Check if we're inside this symbol's scope
        if span.file_id == file_id && offset >= span.start && offset <= span.end {
            // Check children for any symbol with this name that has a type
            for child in metadata.children() {
                let child_meta = child.metadata();

                // Check if this child has the name we're looking for
                if child_meta.name().value == var_name {
                    // Get its type from behaviors
                    if let Some(valued) = child_meta.get_behavior::<ValueBehavior>() {
                        return Some(valued.ty().clone());
                    }
                    if let Some(typed) = child_meta.get_behavior::<TypedBehavior>() {
                        return Some(typed.ty().clone());
                    }
                }

                // Recursively check children (for nested scopes)
                if let Some(ty) = find_in_symbol(&child, file_id, offset, var_name) {
                    return Some(ty);
                }
            }
        }

        None
    }

    let root = model.root();
    for child in root.metadata().children() {
        if let Some(ty) = find_in_symbol(&child, file_id, offset, var_name) {
            return Some(ty);
        }
    }

    None
}
