//! Kestrel Syntax Tree
//!
//! This crate defines the syntax tree representation for the Kestrel language
//! using the `rowan` library for a lossless, resilient syntax tree implementation.
//!
//! # Overview
//!
//! The syntax tree uses `rowan`, which provides:
//! - **Lossless**: Preserves all source text including whitespace and comments
//! - **Immutable**: Syntax trees are immutable and can be safely shared
//! - **Incremental**: Supports efficient incremental parsing
//!
//! # Example
//!
//! ```
//! use kestrel_syntax_tree::{GreenNodeBuilder, SyntaxKind, SyntaxNode};
//!
//! let mut builder = GreenNodeBuilder::new();
//! builder.start_node(SyntaxKind::ModulePath.into());
//! builder.token(SyntaxKind::Identifier.into(), "Main");
//! builder.finish_node();
//!
//! let green = builder.finish();
//! let syntax = SyntaxNode::new_root(green);
//!
//! assert_eq!(syntax.kind(), SyntaxKind::ModulePath);
//! ```

use kestrel_lexer::Token;
use rowan::Language;

// Re-export for use by parsers
pub use rowan::GreenNodeBuilder;

// Define your language for rowan
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntaxKind {
    // ===== Syntax Nodes (Non-terminals) =====
    Root,
    SourceFile,
    DeclarationItem,
    ProtocolDeclaration,
    ProtocolBody,
    StructDeclaration,
    StructBody,
    ExtensionDeclaration,
    ExtensionBody,
    ImportDeclaration,
    ImportItem,
    ModuleDeclaration,
    ModulePath,
    Name,
    TypeAliasDeclaration,
    AliasedType,
    FieldDeclaration,
    FunctionDeclaration,
    InitializerDeclaration,
    FunctionBody,
    ParameterList,
    Parameter,
    ReturnType,
    Visibility,
    StaticModifier,

    // Generic type parameter nodes
    TypeParameterList, // [T, U, V]
    TypeParameter,     // T or T = Default
    TypeArgumentList,  // [Int, String] in type use position
    DefaultType,       // = SomeType

    // Where clause nodes
    WhereClause,         // where T: Proto, U: Other
    TypeBound,           // T: Proto and Proto2
    TypeEquality,        // T.Item == U (associated type equality constraint)
    AssociatedTypeBound, // T.Item: Proto (associated type bound constraint)

    // Associated type nodes
    AssociatedTypeTarget, // Iterator.Item or Add[Int].Output (qualified target in type binding)

    // Conformance nodes
    ConformanceList, // : Proto1, Proto2 (after struct/protocol name)
    ConformanceItem, // Each individual conformance (a type reference)

    // Type nodes
    Ty,
    TyUnit,
    TyNever,
    TyTuple,
    TyFunction,
    TyPath,
    TyArray, // [T] - array/list type
    TyOptional, // T? - optional type
    TyList,
    TyInferred, // _ - inferred type placeholder

    // Path nodes (shared between types and other constructs)
    Path,
    PathElement,

    // Code block and statement nodes
    CodeBlock,           // { statement; statement; expression }
    Statement,           // Wrapper for statement variants
    ExpressionStatement, // expression;
    VariableDeclaration, // let/var name: Type = expr;

    // Expression nodes
    Expression,     // Wrapper for expression variants
    ExprUnit,       // ()
    ExprInteger,    // 42, 0xFF, 0b1010, 0o17
    ExprFloat,      // 3.14, 1.0e10
    ExprString,     // "hello"
    ExprBool,       // true, false
    ExprArray,      // [1, 2, 3]
    ExprTuple,      // (1, 2, 3)
    ExprGrouping,   // (expr)
    ExprPath,       // a.b.c (path expression)
    ExprUnary,      // -expr, !expr (prefix)
    ExprPostfix,    // expr! (postfix)
    ExprBinary,     // a + b, a * b, etc.
    ExprNull,       // null
    ExprCall,       // foo(1, 2) or expr(args)
    ExprAssignment, // lhs = rhs
    ExprIf,         // if condition { then } else { else }
    ElseClause,     // else { ... } or else if ...
    ExprWhile,      // while condition { body }
    ExprLoop,       // loop { body }
    ExprBreak,      // break or break label
    ExprContinue,   // continue or continue label
    ExprReturn,     // return or return expr
    ExprTupleIndex, // tuple.0, tuple.1 (tuple element access)
    LoopLabel,      // label: (before while/loop)
    ArgumentList,   // (arg1, label: arg2, ...)
    Argument,       // Single argument: expr or label: expr

    // ===== Tokens (Terminals) =====
    // Literals
    Identifier,
    String,
    Integer,
    Float,
    Boolean,
    Null,

    // Keywords
    As,
    Break,
    Consuming,
    Continue,
    Else,
    Extend,
    Fileprivate,
    Func,
    If,
    Import,
    Loop,
    Init,
    Internal,
    Let,
    Module,
    Mutating,
    Private,
    Protocol,
    Public,
    Return,
    Static,
    Struct,
    Type,
    Var,
    Where,
    While,

    // Logical keywords
    And,
    Not,
    Or,

    // Braces
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Punctuation
    Semicolon,
    Comma,
    Dot,
    Colon,
    Question,
    Bang,
    Underscore,

    // Operators
    // Multi-character
    DotDotEquals,
    DotDotLess,
    LessLess,
    GreaterGreater,
    LessEquals,
    GreaterEquals,
    EqualsEquals,
    BangEquals,
    QuestionQuestion,
    Arrow,
    // Single-character
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Less,
    Greater,

    // Trivia (whitespace and comments)
    Whitespace,
    LineComment,
    BlockComment,

    // Special
    Error,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl From<Token> for SyntaxKind {
    fn from(token: Token) -> Self {
        match token {
            // Trivia
            Token::Whitespace => SyntaxKind::Whitespace,
            Token::LineComment => SyntaxKind::LineComment,
            Token::BlockComment => SyntaxKind::BlockComment,
            // Literals
            Token::Identifier => SyntaxKind::Identifier,
            Token::String => SyntaxKind::String,
            Token::Integer => SyntaxKind::Integer,
            Token::Float => SyntaxKind::Float,
            Token::Boolean => SyntaxKind::Boolean,
            Token::Null => SyntaxKind::Null,
            // Keywords
            Token::As => SyntaxKind::As,
            Token::Break => SyntaxKind::Break,
            Token::Consuming => SyntaxKind::Consuming,
            Token::Continue => SyntaxKind::Continue,
            Token::Else => SyntaxKind::Else,
            Token::Extend => SyntaxKind::Extend,
            Token::Fileprivate => SyntaxKind::Fileprivate,
            Token::Func => SyntaxKind::Func,
            Token::If => SyntaxKind::If,
            Token::Import => SyntaxKind::Import,
            Token::Init => SyntaxKind::Init,
            Token::Loop => SyntaxKind::Loop,
            Token::Internal => SyntaxKind::Internal,
            Token::Let => SyntaxKind::Let,
            Token::Module => SyntaxKind::Module,
            Token::Mutating => SyntaxKind::Mutating,
            Token::Private => SyntaxKind::Private,
            Token::Protocol => SyntaxKind::Protocol,
            Token::Public => SyntaxKind::Public,
            Token::Return => SyntaxKind::Return,
            Token::Static => SyntaxKind::Static,
            Token::Struct => SyntaxKind::Struct,
            Token::Type => SyntaxKind::Type,
            Token::Var => SyntaxKind::Var,
            Token::Where => SyntaxKind::Where,
            Token::While => SyntaxKind::While,
            // Logical keywords
            Token::And => SyntaxKind::And,
            Token::Not => SyntaxKind::Not,
            Token::Or => SyntaxKind::Or,
            // Braces
            Token::LParen => SyntaxKind::LParen,
            Token::RParen => SyntaxKind::RParen,
            Token::LBrace => SyntaxKind::LBrace,
            Token::RBrace => SyntaxKind::RBrace,
            Token::LBracket => SyntaxKind::LBracket,
            Token::RBracket => SyntaxKind::RBracket,
            // Punctuation
            Token::Semicolon => SyntaxKind::Semicolon,
            Token::Comma => SyntaxKind::Comma,
            Token::Dot => SyntaxKind::Dot,
            Token::Colon => SyntaxKind::Colon,
            Token::Question => SyntaxKind::Question,
            Token::Bang => SyntaxKind::Bang,
            Token::Underscore => SyntaxKind::Underscore,
            // Operators
            Token::DotDotEquals => SyntaxKind::DotDotEquals,
            Token::DotDotLess => SyntaxKind::DotDotLess,
            Token::LessLess => SyntaxKind::LessLess,
            Token::GreaterGreater => SyntaxKind::GreaterGreater,
            Token::LessEquals => SyntaxKind::LessEquals,
            Token::GreaterEquals => SyntaxKind::GreaterEquals,
            Token::EqualsEquals => SyntaxKind::EqualsEquals,
            Token::BangEquals => SyntaxKind::BangEquals,
            Token::QuestionQuestion => SyntaxKind::QuestionQuestion,
            Token::Arrow => SyntaxKind::Arrow,
            Token::Equals => SyntaxKind::Equals,
            Token::Plus => SyntaxKind::Plus,
            Token::Minus => SyntaxKind::Minus,
            Token::Star => SyntaxKind::Star,
            Token::Slash => SyntaxKind::Slash,
            Token::Percent => SyntaxKind::Percent,
            Token::Ampersand => SyntaxKind::Ampersand,
            Token::Pipe => SyntaxKind::Pipe,
            Token::Caret => SyntaxKind::Caret,
            Token::Less => SyntaxKind::Less,
            Token::Greater => SyntaxKind::Greater,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KestrelLanguage;

impl Language for KestrelLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // Constants for pattern matching - suppress naming warnings
        const ROOT: u16 = SyntaxKind::Root as u16;
        const SOURCE_FILE: u16 = SyntaxKind::SourceFile as u16;
        const DECLARATION_ITEM: u16 = SyntaxKind::DeclarationItem as u16;
        const PROTOCOL_DECLARATION: u16 = SyntaxKind::ProtocolDeclaration as u16;
        const PROTOCOL_BODY: u16 = SyntaxKind::ProtocolBody as u16;
        const STRUCT_DECLARATION: u16 = SyntaxKind::StructDeclaration as u16;
        const STRUCT_BODY: u16 = SyntaxKind::StructBody as u16;
        const EXTENSION_DECLARATION: u16 = SyntaxKind::ExtensionDeclaration as u16;
        const EXTENSION_BODY: u16 = SyntaxKind::ExtensionBody as u16;
        const IMPORT_DECLARATION: u16 = SyntaxKind::ImportDeclaration as u16;
        const IMPORT_ITEM: u16 = SyntaxKind::ImportItem as u16;
        const MODULE_DECLARATION: u16 = SyntaxKind::ModuleDeclaration as u16;
        const MODULE_PATH: u16 = SyntaxKind::ModulePath as u16;
        const NAME: u16 = SyntaxKind::Name as u16;
        const TYPE_ALIAS_DECLARATION: u16 = SyntaxKind::TypeAliasDeclaration as u16;
        const ALIASED_TYPE: u16 = SyntaxKind::AliasedType as u16;
        const FIELD_DECLARATION: u16 = SyntaxKind::FieldDeclaration as u16;
        const FUNCTION_DECLARATION: u16 = SyntaxKind::FunctionDeclaration as u16;
        const INITIALIZER_DECLARATION: u16 = SyntaxKind::InitializerDeclaration as u16;
        const FUNCTION_BODY: u16 = SyntaxKind::FunctionBody as u16;
        const PARAMETER_LIST: u16 = SyntaxKind::ParameterList as u16;
        const PARAMETER: u16 = SyntaxKind::Parameter as u16;
        const RETURN_TYPE: u16 = SyntaxKind::ReturnType as u16;
        const VISIBILITY: u16 = SyntaxKind::Visibility as u16;
        const STATIC_MODIFIER: u16 = SyntaxKind::StaticModifier as u16;
        const TYPE_PARAMETER_LIST: u16 = SyntaxKind::TypeParameterList as u16;
        const TYPE_PARAMETER: u16 = SyntaxKind::TypeParameter as u16;
        const TYPE_ARGUMENT_LIST: u16 = SyntaxKind::TypeArgumentList as u16;
        const DEFAULT_TYPE: u16 = SyntaxKind::DefaultType as u16;
        const WHERE_CLAUSE: u16 = SyntaxKind::WhereClause as u16;
        const TYPE_BOUND: u16 = SyntaxKind::TypeBound as u16;
        const TYPE_EQUALITY: u16 = SyntaxKind::TypeEquality as u16;
        const ASSOCIATED_TYPE_BOUND: u16 = SyntaxKind::AssociatedTypeBound as u16;
        const ASSOCIATED_TYPE_TARGET: u16 = SyntaxKind::AssociatedTypeTarget as u16;
        const CONFORMANCE_LIST: u16 = SyntaxKind::ConformanceList as u16;
        const CONFORMANCE_ITEM: u16 = SyntaxKind::ConformanceItem as u16;
        const TY: u16 = SyntaxKind::Ty as u16;
        const TY_UNIT: u16 = SyntaxKind::TyUnit as u16;
        const TY_NEVER: u16 = SyntaxKind::TyNever as u16;
        const TY_TUPLE: u16 = SyntaxKind::TyTuple as u16;
        const TY_FUNCTION: u16 = SyntaxKind::TyFunction as u16;
        const TY_PATH: u16 = SyntaxKind::TyPath as u16;
        const TY_ARRAY: u16 = SyntaxKind::TyArray as u16;
        const TY_LIST: u16 = SyntaxKind::TyList as u16;
        const TY_INFERRED: u16 = SyntaxKind::TyInferred as u16;
        const PATH: u16 = SyntaxKind::Path as u16;
        const PATH_ELEMENT: u16 = SyntaxKind::PathElement as u16;
        const CODE_BLOCK: u16 = SyntaxKind::CodeBlock as u16;
        const STATEMENT: u16 = SyntaxKind::Statement as u16;
        const EXPRESSION_STATEMENT: u16 = SyntaxKind::ExpressionStatement as u16;
        const VARIABLE_DECLARATION: u16 = SyntaxKind::VariableDeclaration as u16;
        const EXPRESSION: u16 = SyntaxKind::Expression as u16;
        const EXPR_UNIT: u16 = SyntaxKind::ExprUnit as u16;
        const EXPR_INTEGER: u16 = SyntaxKind::ExprInteger as u16;
        const EXPR_FLOAT: u16 = SyntaxKind::ExprFloat as u16;
        const EXPR_STRING: u16 = SyntaxKind::ExprString as u16;
        const EXPR_BOOL: u16 = SyntaxKind::ExprBool as u16;
        const EXPR_ARRAY: u16 = SyntaxKind::ExprArray as u16;
        const EXPR_TUPLE: u16 = SyntaxKind::ExprTuple as u16;
        const EXPR_GROUPING: u16 = SyntaxKind::ExprGrouping as u16;
        const EXPR_PATH: u16 = SyntaxKind::ExprPath as u16;
        const EXPR_UNARY: u16 = SyntaxKind::ExprUnary as u16;
        const EXPR_POSTFIX: u16 = SyntaxKind::ExprPostfix as u16;
        const EXPR_BINARY: u16 = SyntaxKind::ExprBinary as u16;
        const EXPR_NULL: u16 = SyntaxKind::ExprNull as u16;
        const EXPR_CALL: u16 = SyntaxKind::ExprCall as u16;
        const EXPR_ASSIGNMENT: u16 = SyntaxKind::ExprAssignment as u16;
        const EXPR_IF: u16 = SyntaxKind::ExprIf as u16;
        const ELSE_CLAUSE: u16 = SyntaxKind::ElseClause as u16;
        const EXPR_WHILE: u16 = SyntaxKind::ExprWhile as u16;
        const EXPR_LOOP: u16 = SyntaxKind::ExprLoop as u16;
        const EXPR_BREAK: u16 = SyntaxKind::ExprBreak as u16;
        const EXPR_CONTINUE: u16 = SyntaxKind::ExprContinue as u16;
        const EXPR_RETURN: u16 = SyntaxKind::ExprReturn as u16;
        const EXPR_TUPLE_INDEX: u16 = SyntaxKind::ExprTupleIndex as u16;
        const LOOP_LABEL: u16 = SyntaxKind::LoopLabel as u16;
        const ARGUMENT_LIST: u16 = SyntaxKind::ArgumentList as u16;
        const ARGUMENT: u16 = SyntaxKind::Argument as u16;
        const IDENTIFIER: u16 = SyntaxKind::Identifier as u16;
        const STRING: u16 = SyntaxKind::String as u16;
        const INTEGER: u16 = SyntaxKind::Integer as u16;
        const FLOAT: u16 = SyntaxKind::Float as u16;
        const BOOLEAN: u16 = SyntaxKind::Boolean as u16;
        const NULL: u16 = SyntaxKind::Null as u16;
        const AS: u16 = SyntaxKind::As as u16;
        const BREAK: u16 = SyntaxKind::Break as u16;
        const CONSUMING: u16 = SyntaxKind::Consuming as u16;
        const CONTINUE: u16 = SyntaxKind::Continue as u16;
        const ELSE: u16 = SyntaxKind::Else as u16;
        const EXTEND: u16 = SyntaxKind::Extend as u16;
        const FILEPRIVATE: u16 = SyntaxKind::Fileprivate as u16;
        const FUNC: u16 = SyntaxKind::Func as u16;
        const IF: u16 = SyntaxKind::If as u16;
        const IMPORT: u16 = SyntaxKind::Import as u16;
        const INIT: u16 = SyntaxKind::Init as u16;
        const LOOP: u16 = SyntaxKind::Loop as u16;
        const INTERNAL: u16 = SyntaxKind::Internal as u16;
        const LET: u16 = SyntaxKind::Let as u16;
        const MODULE: u16 = SyntaxKind::Module as u16;
        const MUTATING: u16 = SyntaxKind::Mutating as u16;
        const PRIVATE: u16 = SyntaxKind::Private as u16;
        const PROTOCOL: u16 = SyntaxKind::Protocol as u16;
        const PUBLIC: u16 = SyntaxKind::Public as u16;
        const RETURN: u16 = SyntaxKind::Return as u16;
        const STATIC: u16 = SyntaxKind::Static as u16;
        const STRUCT: u16 = SyntaxKind::Struct as u16;
        const TYPE: u16 = SyntaxKind::Type as u16;
        const VAR: u16 = SyntaxKind::Var as u16;
        const WHERE: u16 = SyntaxKind::Where as u16;
        const WHILE: u16 = SyntaxKind::While as u16;
        // Logical keywords
        const AND: u16 = SyntaxKind::And as u16;
        const NOT: u16 = SyntaxKind::Not as u16;
        const OR: u16 = SyntaxKind::Or as u16;
        const LPAREN: u16 = SyntaxKind::LParen as u16;
        const RPAREN: u16 = SyntaxKind::RParen as u16;
        const LBRACE: u16 = SyntaxKind::LBrace as u16;
        const RBRACE: u16 = SyntaxKind::RBrace as u16;
        const LBRACKET: u16 = SyntaxKind::LBracket as u16;
        const RBRACKET: u16 = SyntaxKind::RBracket as u16;
        const SEMICOLON: u16 = SyntaxKind::Semicolon as u16;
        const COMMA: u16 = SyntaxKind::Comma as u16;
        const DOT: u16 = SyntaxKind::Dot as u16;
        const COLON: u16 = SyntaxKind::Colon as u16;
        const QUESTION: u16 = SyntaxKind::Question as u16;
        const BANG: u16 = SyntaxKind::Bang as u16;
        const UNDERSCORE: u16 = SyntaxKind::Underscore as u16;
        // Operators
        const DOT_DOT_EQUALS: u16 = SyntaxKind::DotDotEquals as u16;
        const DOT_DOT_LESS: u16 = SyntaxKind::DotDotLess as u16;
        const LESS_LESS: u16 = SyntaxKind::LessLess as u16;
        const GREATER_GREATER: u16 = SyntaxKind::GreaterGreater as u16;
        const LESS_EQUALS: u16 = SyntaxKind::LessEquals as u16;
        const GREATER_EQUALS: u16 = SyntaxKind::GreaterEquals as u16;
        const EQUALS_EQUALS: u16 = SyntaxKind::EqualsEquals as u16;
        const BANG_EQUALS: u16 = SyntaxKind::BangEquals as u16;
        const QUESTION_QUESTION: u16 = SyntaxKind::QuestionQuestion as u16;
        const ARROW: u16 = SyntaxKind::Arrow as u16;
        const EQUALS: u16 = SyntaxKind::Equals as u16;
        const PLUS: u16 = SyntaxKind::Plus as u16;
        const MINUS: u16 = SyntaxKind::Minus as u16;
        const STAR: u16 = SyntaxKind::Star as u16;
        const SLASH: u16 = SyntaxKind::Slash as u16;
        const PERCENT: u16 = SyntaxKind::Percent as u16;
        const AMPERSAND: u16 = SyntaxKind::Ampersand as u16;
        const PIPE: u16 = SyntaxKind::Pipe as u16;
        const CARET: u16 = SyntaxKind::Caret as u16;
        const LESS: u16 = SyntaxKind::Less as u16;
        const GREATER: u16 = SyntaxKind::Greater as u16;
        const WHITESPACE: u16 = SyntaxKind::Whitespace as u16;
        const LINE_COMMENT: u16 = SyntaxKind::LineComment as u16;
        const BLOCK_COMMENT: u16 = SyntaxKind::BlockComment as u16;

        match raw.0 {
            ROOT => SyntaxKind::Root,
            SOURCE_FILE => SyntaxKind::SourceFile,
            DECLARATION_ITEM => SyntaxKind::DeclarationItem,
            PROTOCOL_DECLARATION => SyntaxKind::ProtocolDeclaration,
            PROTOCOL_BODY => SyntaxKind::ProtocolBody,
            STRUCT_DECLARATION => SyntaxKind::StructDeclaration,
            STRUCT_BODY => SyntaxKind::StructBody,
            EXTENSION_DECLARATION => SyntaxKind::ExtensionDeclaration,
            EXTENSION_BODY => SyntaxKind::ExtensionBody,
            IMPORT_DECLARATION => SyntaxKind::ImportDeclaration,
            IMPORT_ITEM => SyntaxKind::ImportItem,
            MODULE_DECLARATION => SyntaxKind::ModuleDeclaration,
            MODULE_PATH => SyntaxKind::ModulePath,
            NAME => SyntaxKind::Name,
            TYPE_ALIAS_DECLARATION => SyntaxKind::TypeAliasDeclaration,
            ALIASED_TYPE => SyntaxKind::AliasedType,
            FIELD_DECLARATION => SyntaxKind::FieldDeclaration,
            FUNCTION_DECLARATION => SyntaxKind::FunctionDeclaration,
            INITIALIZER_DECLARATION => SyntaxKind::InitializerDeclaration,
            FUNCTION_BODY => SyntaxKind::FunctionBody,
            PARAMETER_LIST => SyntaxKind::ParameterList,
            PARAMETER => SyntaxKind::Parameter,
            RETURN_TYPE => SyntaxKind::ReturnType,
            VISIBILITY => SyntaxKind::Visibility,
            STATIC_MODIFIER => SyntaxKind::StaticModifier,
            TYPE_PARAMETER_LIST => SyntaxKind::TypeParameterList,
            TYPE_PARAMETER => SyntaxKind::TypeParameter,
            TYPE_ARGUMENT_LIST => SyntaxKind::TypeArgumentList,
            DEFAULT_TYPE => SyntaxKind::DefaultType,
            WHERE_CLAUSE => SyntaxKind::WhereClause,
            TYPE_BOUND => SyntaxKind::TypeBound,
            TYPE_EQUALITY => SyntaxKind::TypeEquality,
            ASSOCIATED_TYPE_BOUND => SyntaxKind::AssociatedTypeBound,
            ASSOCIATED_TYPE_TARGET => SyntaxKind::AssociatedTypeTarget,
            CONFORMANCE_LIST => SyntaxKind::ConformanceList,
            CONFORMANCE_ITEM => SyntaxKind::ConformanceItem,
            TY => SyntaxKind::Ty,
            TY_UNIT => SyntaxKind::TyUnit,
            TY_NEVER => SyntaxKind::TyNever,
            TY_TUPLE => SyntaxKind::TyTuple,
            TY_FUNCTION => SyntaxKind::TyFunction,
            TY_PATH => SyntaxKind::TyPath,
            TY_ARRAY => SyntaxKind::TyArray,
            TY_LIST => SyntaxKind::TyList,
            TY_INFERRED => SyntaxKind::TyInferred,
            PATH => SyntaxKind::Path,
            PATH_ELEMENT => SyntaxKind::PathElement,
            CODE_BLOCK => SyntaxKind::CodeBlock,
            STATEMENT => SyntaxKind::Statement,
            EXPRESSION_STATEMENT => SyntaxKind::ExpressionStatement,
            VARIABLE_DECLARATION => SyntaxKind::VariableDeclaration,
            EXPRESSION => SyntaxKind::Expression,
            EXPR_UNIT => SyntaxKind::ExprUnit,
            EXPR_INTEGER => SyntaxKind::ExprInteger,
            EXPR_FLOAT => SyntaxKind::ExprFloat,
            EXPR_STRING => SyntaxKind::ExprString,
            EXPR_BOOL => SyntaxKind::ExprBool,
            EXPR_ARRAY => SyntaxKind::ExprArray,
            EXPR_TUPLE => SyntaxKind::ExprTuple,
            EXPR_GROUPING => SyntaxKind::ExprGrouping,
            EXPR_PATH => SyntaxKind::ExprPath,
            EXPR_UNARY => SyntaxKind::ExprUnary,
            EXPR_POSTFIX => SyntaxKind::ExprPostfix,
            EXPR_BINARY => SyntaxKind::ExprBinary,
            EXPR_NULL => SyntaxKind::ExprNull,
            EXPR_CALL => SyntaxKind::ExprCall,
            EXPR_ASSIGNMENT => SyntaxKind::ExprAssignment,
            EXPR_IF => SyntaxKind::ExprIf,
            ELSE_CLAUSE => SyntaxKind::ElseClause,
            EXPR_WHILE => SyntaxKind::ExprWhile,
            EXPR_LOOP => SyntaxKind::ExprLoop,
            EXPR_BREAK => SyntaxKind::ExprBreak,
            EXPR_CONTINUE => SyntaxKind::ExprContinue,
            EXPR_RETURN => SyntaxKind::ExprReturn,
            EXPR_TUPLE_INDEX => SyntaxKind::ExprTupleIndex,
            LOOP_LABEL => SyntaxKind::LoopLabel,
            ARGUMENT_LIST => SyntaxKind::ArgumentList,
            ARGUMENT => SyntaxKind::Argument,
            IDENTIFIER => SyntaxKind::Identifier,
            STRING => SyntaxKind::String,
            INTEGER => SyntaxKind::Integer,
            FLOAT => SyntaxKind::Float,
            BOOLEAN => SyntaxKind::Boolean,
            NULL => SyntaxKind::Null,
            AS => SyntaxKind::As,
            BREAK => SyntaxKind::Break,
            CONSUMING => SyntaxKind::Consuming,
            CONTINUE => SyntaxKind::Continue,
            ELSE => SyntaxKind::Else,
            EXTEND => SyntaxKind::Extend,
            FILEPRIVATE => SyntaxKind::Fileprivate,
            FUNC => SyntaxKind::Func,
            IF => SyntaxKind::If,
            IMPORT => SyntaxKind::Import,
            INIT => SyntaxKind::Init,
            LOOP => SyntaxKind::Loop,
            INTERNAL => SyntaxKind::Internal,
            LET => SyntaxKind::Let,
            MODULE => SyntaxKind::Module,
            MUTATING => SyntaxKind::Mutating,
            PRIVATE => SyntaxKind::Private,
            PROTOCOL => SyntaxKind::Protocol,
            PUBLIC => SyntaxKind::Public,
            RETURN => SyntaxKind::Return,
            STATIC => SyntaxKind::Static,
            STRUCT => SyntaxKind::Struct,
            TYPE => SyntaxKind::Type,
            VAR => SyntaxKind::Var,
            WHERE => SyntaxKind::Where,
            WHILE => SyntaxKind::While,
            // Logical keywords
            AND => SyntaxKind::And,
            NOT => SyntaxKind::Not,
            OR => SyntaxKind::Or,
            LPAREN => SyntaxKind::LParen,
            RPAREN => SyntaxKind::RParen,
            LBRACE => SyntaxKind::LBrace,
            RBRACE => SyntaxKind::RBrace,
            LBRACKET => SyntaxKind::LBracket,
            RBRACKET => SyntaxKind::RBracket,
            SEMICOLON => SyntaxKind::Semicolon,
            COMMA => SyntaxKind::Comma,
            DOT => SyntaxKind::Dot,
            COLON => SyntaxKind::Colon,
            QUESTION => SyntaxKind::Question,
            BANG => SyntaxKind::Bang,
            UNDERSCORE => SyntaxKind::Underscore,
            // Operators
            DOT_DOT_EQUALS => SyntaxKind::DotDotEquals,
            DOT_DOT_LESS => SyntaxKind::DotDotLess,
            LESS_LESS => SyntaxKind::LessLess,
            GREATER_GREATER => SyntaxKind::GreaterGreater,
            LESS_EQUALS => SyntaxKind::LessEquals,
            GREATER_EQUALS => SyntaxKind::GreaterEquals,
            EQUALS_EQUALS => SyntaxKind::EqualsEquals,
            BANG_EQUALS => SyntaxKind::BangEquals,
            QUESTION_QUESTION => SyntaxKind::QuestionQuestion,
            ARROW => SyntaxKind::Arrow,
            EQUALS => SyntaxKind::Equals,
            PLUS => SyntaxKind::Plus,
            MINUS => SyntaxKind::Minus,
            STAR => SyntaxKind::Star,
            SLASH => SyntaxKind::Slash,
            PERCENT => SyntaxKind::Percent,
            AMPERSAND => SyntaxKind::Ampersand,
            PIPE => SyntaxKind::Pipe,
            CARET => SyntaxKind::Caret,
            LESS => SyntaxKind::Less,
            GREATER => SyntaxKind::Greater,
            WHITESPACE => SyntaxKind::Whitespace,
            LINE_COMMENT => SyntaxKind::LineComment,
            BLOCK_COMMENT => SyntaxKind::BlockComment,
            _ => SyntaxKind::Error,
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<KestrelLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<KestrelLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<KestrelLanguage>;

pub mod imports;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_kind_conversion() {
        // Test that Token to SyntaxKind conversion works
        assert_eq!(
            SyntaxKind::from(kestrel_lexer::Token::Module),
            SyntaxKind::Module
        );
        assert_eq!(
            SyntaxKind::from(kestrel_lexer::Token::Identifier),
            SyntaxKind::Identifier
        );
        assert_eq!(SyntaxKind::from(kestrel_lexer::Token::Dot), SyntaxKind::Dot);
    }

    #[test]
    fn test_basic_tree() {
        // Test building a simple syntax tree
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::Root.into());
        builder.token(SyntaxKind::Identifier.into(), "test");
        builder.finish_node();

        let green = builder.finish();
        let root = SyntaxNode::new_root(green);

        assert_eq!(root.kind(), SyntaxKind::Root);
    }
}
