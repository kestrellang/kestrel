use kestrel_span2::Span;

use crate::{SyntaxKind, SyntaxNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportItemSyntax {
    pub name: String,
    pub alias: Option<String>,
    pub name_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDeclarationSyntax {
    pub module_path: Vec<(String, Span)>,
    pub module_path_span: Span,
    pub alias: Option<String>,
    pub items: Vec<ImportItemSyntax>,
}

pub fn extract_import_declaration(
    syntax: &SyntaxNode,
    file_id: usize,
) -> Option<ImportDeclarationSyntax> {
    if syntax.kind() != SyntaxKind::ImportDeclaration {
        return None;
    }

    let module_path_node = syntax
        .children()
        .find(|node| node.kind() == SyntaxKind::ModulePath)?;
    let module_path = module_path_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .filter(|tok| tok.kind() == SyntaxKind::Identifier)
        .map(|tok| {
            let range = tok.text_range();
            let start: usize = range.start().into();
            let end: usize = range.end().into();
            (tok.text().to_string(), Span::new(file_id, start..end))
        })
        .collect::<Vec<_>>();

    let module_path_range = module_path_node.text_range();
    let module_path_span: Span = Span::new(
        file_id,
        (module_path_range.start().into())..(module_path_range.end().into()),
    );

    let has_items = syntax
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .any(|tok| tok.kind() == SyntaxKind::LParen);

    let alias = if has_items {
        None
    } else {
        let mut found_as = false;
        syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find_map(|tok| {
                if found_as && tok.kind() == SyntaxKind::Identifier {
                    return Some(tok.text().to_string());
                }
                if tok.kind() == SyntaxKind::As {
                    found_as = true;
                }
                None
            })
    };

    let items = syntax
        .children()
        .filter(|node| node.kind() == SyntaxKind::ImportItem)
        .filter_map(|item_node| {
            let (name, name_span) = item_node
                .children_with_tokens()
                .filter_map(|elem| elem.into_token())
                .find_map(|tok| {
                    if tok.kind() != SyntaxKind::Identifier {
                        return None;
                    }
                    let range = tok.text_range();
                    let start: usize = range.start().into();
                    let end: usize = range.end().into();
                    Some((tok.text().to_string(), Span::new(file_id, start..end)))
                })?;

            let mut found_as = false;
            let alias = item_node
                .children_with_tokens()
                .filter_map(|elem| elem.into_token())
                .find_map(|tok| {
                    if found_as && tok.kind() == SyntaxKind::Identifier {
                        return Some(tok.text().to_string());
                    }
                    if tok.kind() == SyntaxKind::As {
                        found_as = true;
                    }
                    None
                });

            Some(ImportItemSyntax {
                name,
                alias,
                name_span,
            })
        })
        .collect::<Vec<_>>();

    Some(ImportDeclarationSyntax {
        module_path,
        module_path_span,
        alias,
        items,
    })
}
