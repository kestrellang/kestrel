//! `textDocument/semanticTokens/full` — entity-aware colored highlighting.
//!
//! Two-phase approach: first, build an override map by walking all HIR bodies
//! and declaration entities in the file to classify each identifier by its
//! resolved entity. Then encode the lexer token stream, consulting the override
//! map for identifiers. Unresolved identifiers fall back to a PascalCase
//! heuristic (uppercase → type, lowercase → variable).

use std::collections::HashMap;

use kestrel_ast_builder::{Body, CstNode, DeclSpan, FileId, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::HirExpr;
use kestrel_hir_lower::LowerBody;
use kestrel_lexer::Token;
use kestrel_syntax_tree::utils::get_name_span;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensParams, SemanticTokensResult,
};

use crate::position::LineIndex;
use crate::semantic;
use crate::server::{SharedState, url_to_path};

/// Token-type legend, indexed by the `tokenType` field in the wire format.
/// Order matters — clients use these indices.
pub const LEGEND: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,   // 0
    SemanticTokenType::TYPE,      // 1
    SemanticTokenType::FUNCTION,  // 2
    SemanticTokenType::VARIABLE,  // 3
    SemanticTokenType::PROPERTY,  // 4
    SemanticTokenType::NAMESPACE, // 5
    SemanticTokenType::COMMENT,   // 6
    SemanticTokenType::STRING,    // 7
    SemanticTokenType::NUMBER,    // 8
    SemanticTokenType::OPERATOR,  // 9
];

const KEYWORD: u32 = 0;
const TYPE: u32 = 1;
const FUNCTION: u32 = 2;
const VARIABLE: u32 = 3;
const PROPERTY: u32 = 4;
const NAMESPACE: u32 = 5;
const COMMENT: u32 = 6;
const STRING: u32 = 7;
const NUMBER: u32 = 8;
const OPERATOR: u32 = 9;

pub async fn handle(
    state: SharedState,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = params.text_document.uri;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, line_index)
    };

    let data = handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<SemanticToken>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let raw_tokens = compiler.lex(file_entity);
                let source = line_index.text();
                let world = compiler.world();
                let root = compiler.root();
                let overrides = build_entity_overrides(world, root, file_entity, source);
                Some(encode(&raw_tokens, source, &line_index, &overrides))
            },
        )
        .await??;

    Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
    }))
}

/// Map a `NodeKind` to a semantic token type index.
fn node_kind_to_token_type(kind: &NodeKind) -> u32 {
    match kind {
        NodeKind::Function | NodeKind::Initializer | NodeKind::Subscript | NodeKind::Deinit => {
            FUNCTION
        },
        NodeKind::Struct
        | NodeKind::Enum
        | NodeKind::Protocol
        | NodeKind::TypeAlias
        | NodeKind::TypeParameter => TYPE,
        NodeKind::Field | NodeKind::EnumCase => PROPERTY,
        NodeKind::Module => NAMESPACE,
        NodeKind::Extension | NodeKind::Import | NodeKind::Setter | NodeKind::ParamDefault => {
            VARIABLE
        },
    }
}

fn resolved_entity_token_type(world: &World, entity: Entity) -> u32 {
    world
        .get::<NodeKind>(entity)
        .map(|k| node_kind_to_token_type(k))
        .unwrap_or(VARIABLE)
}

/// Walk declarations and HIR bodies in the file, building a map from
/// identifier byte offset → semantic token type.
fn build_entity_overrides(
    world: &World,
    root: Entity,
    file_entity: Entity,
    source: &str,
) -> HashMap<usize, u32> {
    let mut overrides = HashMap::new();
    let ctx = world.query_context();

    // Phase 1: declaration names (entities with CstNode in this file).
    for (entity, fid) in world.iter_component::<FileId>() {
        if fid.0 != file_entity {
            continue;
        }
        let Some(kind) = world.get::<NodeKind>(entity) else {
            continue;
        };
        let Some(cst) = world.get::<CstNode>(entity) else {
            continue;
        };
        let Some(decl_span) = world.get::<DeclSpan>(entity) else {
            continue;
        };
        if let Some(name_span) = get_name_span(&cst.0, decl_span.0.file_id) {
            overrides.insert(name_span.start, node_kind_to_token_type(kind));
        }
    }

    // Phase 2: body expressions — classify each identifier reference.
    for (body_entity, _) in world.iter_component::<Body>() {
        let Some(fid) = world.get::<FileId>(body_entity) else {
            continue;
        };
        if fid.0 != file_entity {
            continue;
        }
        let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        }) else {
            continue;
        };
        let typed = ctx.query(InferBody {
            entity: body_entity,
            root,
        });

        for (id, expr) in hir.exprs.iter() {
            match expr {
                HirExpr::Def(entity, _, span) => {
                    overrides.insert(span.start, resolved_entity_token_type(world, *entity));
                },
                HirExpr::Local(_, span) => {
                    overrides.insert(span.start, VARIABLE);
                },
                HirExpr::OverloadSet { span, .. } => {
                    overrides.insert(span.start, FUNCTION);
                },
                HirExpr::Field { span, .. } => {
                    if let Some(typed) = &typed {
                        if let Some(&resolved) = typed.resolutions.get(&id) {
                            let ttype = resolved_entity_token_type(world, resolved);
                            let text = source.get(span.start..span.end).unwrap_or("");
                            if let Some(dot_pos) = text.rfind('.') {
                                overrides.insert(span.start + dot_pos + 1, ttype);
                            }
                        }
                    }
                },
                HirExpr::MethodCall { span, .. } => {
                    if let Some(typed) = &typed {
                        if let Some(&_resolved) = typed.resolutions.get(&id) {
                            let text = source.get(span.start..span.end).unwrap_or("");
                            let before_paren = text.find('(').unwrap_or(text.len());
                            if let Some(dot_pos) = text[..before_paren].rfind('.') {
                                overrides.insert(span.start + dot_pos + 1, FUNCTION);
                            }
                        }
                    }
                },
                HirExpr::ProtocolCall { span, .. } => {
                    if let Some(typed) = &typed {
                        if let Some(&_resolved) = typed.resolutions.get(&id) {
                            let text = source.get(span.start..span.end).unwrap_or("");
                            let before_paren = text.find('(').unwrap_or(text.len());
                            if let Some(dot_pos) = text[..before_paren].rfind('.') {
                                overrides.insert(span.start + dot_pos + 1, FUNCTION);
                            }
                        }
                    }
                },
                HirExpr::ImplicitMember { span, .. } => {
                    if let Some(typed) = &typed {
                        if let Some(&resolved) = typed.resolutions.get(&id) {
                            let ttype = resolved_entity_token_type(world, resolved);
                            let text = source.get(span.start..span.end).unwrap_or("");
                            if let Some(dot_pos) = text.find('.') {
                                overrides.insert(span.start + dot_pos + 1, ttype);
                            }
                        }
                    }
                },
                _ => {},
            }
        }
    }

    overrides
}

fn classify(token: Token, lexeme: &str, span_start: usize, overrides: &HashMap<usize, u32>) -> Option<u32> {
    use Token::*;
    // `Option::Some` because `use Token::*` brings `Token::Some` into scope.
    Option::Some(match token {
        LineComment | BlockComment => COMMENT,
        Whitespace | Newline => return None,

        String | Char | RawString => STRING,
        Integer | Float => NUMBER,
        Boolean | Null => KEYWORD,

        Extend | Fileprivate | Func | Import | Deinit | Init | Internal | Let | Module
        | Mutating | Private | Protocol | Public | Static | Struct | Type | Var | Where | Enum
        | Case | Indirect | And | Not | Or | As | Break | Consuming | Continue | Else | For
        | If | In | Loop | Return | Throw | Try | Throws | While | Match | Guard | Get | Set
        | Subscript | Some => KEYWORD,

        // Entity-aware classification for identifiers.
        Identifier => {
            if let Option::Some(&ttype) = overrides.get(&span_start) {
                ttype
            } else if lexeme.chars().next().is_some_and(|c| c.is_uppercase()) {
                TYPE
            } else {
                VARIABLE
            }
        },
        Underscore => VARIABLE,

        _ => OPERATOR,
    })
}

fn encode(
    raw: &[kestrel_lexer::SpannedToken],
    source: &str,
    idx: &LineIndex,
    overrides: &HashMap<usize, u32>,
) -> Vec<SemanticToken> {
    let mut out = Vec::with_capacity(raw.len());
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for spanned in raw {
        let token = spanned.value.clone();
        let span = &spanned.span;
        let lexeme = source.get(span.start..span.end).unwrap_or("");
        let Option::Some(ttype) = classify(token, lexeme, span.start, overrides) else {
            continue;
        };

        let start_pos = idx.offset_to_position(span.start);
        let end_pos = idx.offset_to_position(span.end);
        if start_pos.line != end_pos.line {
            continue;
        }
        let length = end_pos.character - start_pos.character;
        if length == 0 {
            continue;
        }

        let delta_line = start_pos.line - prev_line;
        let delta_start = if delta_line == 0 {
            start_pos.character - prev_start
        } else {
            start_pos.character
        };

        out.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: ttype,
            token_modifiers_bitset: 0,
        });

        prev_line = start_pos.line;
        prev_start = start_pos.character;
    }
    out
}
