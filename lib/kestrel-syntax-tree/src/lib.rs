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

    // Attribute nodes
    Attribute,     // @name or @name(args)
    AttributeList, // Zero or more attributes before a declaration
    AttributeArgs, // (arg, arg, ...) argument list
    AttributeArg,  // Single argument: value or label: value

    ProtocolDeclaration,
    ProtocolBody,
    StructDeclaration,
    StructBody,
    ExtensionDeclaration,
    ExtensionBody,
    EnumDeclaration,
    EnumBody,
    EnumCaseDeclaration,
    EnumCaseParameter,
    EnumCaseParameterList,
    IndirectModifier,
    ImportDeclaration,
    ImportItem,
    ModuleDeclaration,
    ModulePath,
    Name,
    TypeAliasDeclaration,
    AliasedType,
    FieldDeclaration,
    GetterClause,      // get { ... }
    SetterClause,      // set { ... }
    PropertyAccessors, // { get { } set { } } or { get } { get set }
    FunctionDeclaration,
    InitializerDeclaration,
    InitEffect, // ? or throws E after init params
    DeinitDeclaration,
    SubscriptDeclaration,
    SubscriptBody,
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
    DefaultType,       // = SomeType (for type parameters)
    DefaultValue,      // = expression (for function parameters)

    // Where clause nodes
    WhereClause,         // where T: Proto, U: Other
    TypeBound,           // T: Proto and Proto2
    TypeEquality,        // T.Item == U (associated type equality constraint)
    AssociatedTypeBound, // T.Item: Proto (associated type bound constraint)

    // Associated type nodes
    AssociatedTypeTarget, // Iterator.Item or Add[Int].Output (qualified target in type binding)

    // Conformance nodes
    ConformanceList,     // : Proto1, Proto2 (after struct/protocol name)
    ConformanceItem,     // Each individual conformance (a type reference)
    NegativeConformance, // not Proto (opt-out of implicit conformance)

    // Type nodes
    Ty,
    TyUnit,
    TyNever,
    TyTuple,
    TyFunction,
    TyPath,
    TyArray,      // [T] - array/list type
    TyDictionary, // [K: V] - dictionary/map type
    TyOptional,   // T? - optional type
    TyResult,     // T throws E - result type
    TyList,
    TyInferred, // _ - inferred type placeholder
    TySome,     // some P - opaque type

    // Path nodes (shared between types and other constructs)
    Path,
    PathElement,

    // Code block and statement nodes
    CodeBlock,           // { statement; statement; expression }
    Statement,           // Wrapper for statement variants
    ExpressionStatement, // expression;
    VariableDeclaration, // let/var name: Type = expr;
    GuardStatement,      // guard <condition> else { block }
    DeinitStatement,     // deinit identifier; - explicit destructor call
    GuardCondition,      // let pattern = expr (in guard condition chain)

    // Expression nodes
    Expression,               // Wrapper for expression variants
    ExprUnit,                 // ()
    ExprInteger,              // 42, 0xFF, 0b1010, 0o17
    ExprFloat,                // 3.14, 1.0e10
    ExprString,               // "hello"
    ExprRawString,            // """hello""" (raw/multi-line string)
    ExprInterpolatedString,   // "Hello \(name)!" - string with interpolations
    StringLiteralPart,        // Literal text segment in interpolated string
    StringInterpolation,      // \(expr) or \(expr:format) segment
    FormatSpecifier,          // :format_spec in string interpolation
    ExprChar,                 // 'a', '\n', '\u{1F600}'
    ExprBool,                 // true, false
    ExprArray,                // [1, 2, 3]
    ExprDictionary,           // ["key": value, ...]
    DictionaryEntry,          // key: value (single entry in dictionary literal)
    ExprTuple,                // (1, 2, 3)
    ExprGrouping,             // (expr)
    ExprPath,                 // a.b.c (path expression)
    ExprUnary,                // -expr, !expr (prefix)
    ExprPostfix,              // expr! (postfix)
    ExprBinary,               // a + b, a * b, etc.
    ExprNull,                 // null
    ExprCall,                 // foo(1, 2) or expr(args)
    ExprAssignment,           // lhs = rhs
    ExprCompoundAssignment,   // lhs += rhs, lhs -= rhs, etc.
    ExprIf,                   // if condition { then } else { else }
    IfLetCondition,           // let pattern = expr (in if-let condition)
    ElseClause,               // else { ... } or else if ...
    ExprWhile,                // while condition { body }
    WhileLetCondition,        // let pattern = expr (in while-let condition)
    ExprFor,                  // for pattern in iterable { body }
    ForPattern,               // pattern in `for pattern in ...`
    ForIterable,              // iterable expression in `for ... in expression`
    ExprLoop,                 // loop { body }
    ExprBreak,                // break or break label
    ExprContinue,             // continue or continue label
    ExprReturn,               // return or return expr
    ExprThrow,                // throw expr
    ExprTry,                  // try expr
    ExprTupleIndex,           // tuple.0, tuple.1 (tuple element access)
    ExprClosure,              // { params in body } or { body }
    ClosureParams,            // (param, param) in closure
    ClosureParam,             // Single closure parameter: name or name: Type
    LoopLabel,                // label: (before while/loop)
    ArgumentList,             // (arg1, label: arg2, ...)
    Argument,                 // Single argument: expr or label: expr
    ExprImplicitMemberAccess, // .Case or .Case(args)
    ExprMatch,                // match scrutinee { arms }
    MatchArm,                 // pattern => expression
    MatchArmGuard,            // if condition (guard clause in match arm)

    // Pattern nodes
    Pattern,             // Root pattern wrapper
    WildcardPattern,     // _
    BindingPattern,      // name or var name
    TuplePattern,        // (p1, p2, ...)
    TuplePatternElement, // Single element in tuple pattern
    LiteralPattern,      // 42, "hello", 'c', true
    RangePattern,        // 0..=9 or 0..<10 (range pattern)
    EnumPattern,         // .Case or .Case(args)
    EnumPatternArg,      // Single arg in enum pattern: label or label: pattern
    NullPattern,         // null (sugar for .None on Optional)
    SomePattern,         // some PAT (sugar for .Some(PAT) on Optional)
    StructPattern,       // Point { x, y } or Point { x: a, y: b }
    StructPatternField,  // Single field: name or name: pattern
    StructPatternRest,   // .. (ignore remaining fields)
    ArrayPattern,        // [a, b, ..rest]
    ArrayPatternElement, // Single element in array pattern
    ArrayPatternRest,    // ..rest or .. (rest pattern in arrays)
    AtPattern,           // name @ pattern (binds name while matching pattern)
    RestPattern,         // .. (rest pattern in tuples)
    OrPattern,           // p1 or p2 or ... (or-pattern)
    ErrorPattern,        // Error recovery

    // ===== Tokens (Terminals) =====
    // Literals
    Identifier,
    String,
    RawString, // """...""" raw string literal
    Char,      // 'a' character literal
    Integer,
    Float,
    Boolean,
    Null,
    Some,

    // Keywords
    As,
    Break,
    Case,
    Consuming,
    Continue,
    Deinit,
    Else,
    Enum,
    Extend,
    For,
    Fileprivate,
    Func,
    If,
    Import,
    Indirect,
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
    Throw,
    Try,
    Throws,
    Static,
    Struct,
    Type,
    Var,
    Where,
    While,
    In,
    Match,
    Guard,
    Get,
    Set,
    Subscript,

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
    DotDot,
    LessLessEquals,       // <<=
    GreaterGreaterEquals, // >>=
    LessLess,
    GreaterGreater,
    LessEquals,
    GreaterEquals,
    EqualsEquals,
    BangEquals,
    QuestionQuestion,
    Arrow,
    FatArrow,
    // Compound assignment (2-char)
    PlusEquals,      // +=
    MinusEquals,     // -=
    StarEquals,      // *=
    SlashEquals,     // /=
    PercentEquals,   // %=
    AmpersandEquals, // &=
    PipeEquals,      // |=
    CaretEquals,     // ^=
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
    At,

    // Trivia (whitespace and comments)
    Whitespace,
    Newline,
    LineComment,
    BlockComment,

    // Special
    Error,
    /// Wrapper node around a zero-width synthesized token, emitted by the
    /// parser when a required token is absent (e.g. the identifier after
    /// `foo.`). The single child token retains its intended `SyntaxKind` so
    /// downstream consumers still pattern-match cleanly; the `Missing` parent
    /// is what flags the absence.
    Missing,

    // New kinds are appended here (NOT grouped with their family above):
    // rowan green trees store kinds as raw u16 discriminants, so inserting
    // mid-enum would shift every later kind and corrupt cached trees.
    TyRef,             // &T - shared reference type (parsed, rejected until stage 1)
    TyMutRef,          // &mutating T - mutable reference type
    RefClause,         // ref { ... } place accessor (stage 1.5)
    MutatingRefClause, // mutating ref { ... } place accessor (stage 1.5)
    RefBindingPattern, // &name / &mutating name binder pattern (stage 1.5 item 2)
    // Interpolated strings (modal lexing; see kestrel-lexer `modal.rs`)
    StringStart,    // `"` or `"""` opening an interpolated string
    StringFragment, // literal text between holes
    InterpStart,    // `\(`
    InterpEnd,      // `)` closing a hole
    FormatSpec,     // format spec text after a hole's `:`
    StringEnd,      // `"` or `"""` closing an interpolated string

    /// Not a syntax kind — the end-of-enum marker.
    ///
    /// Its discriminant is the variant count, which is what lets
    /// `syntax_kind_table_round_trips` prove `SyntaxKind::ALL` is **complete**
    /// rather than merely self-consistent (a truncated table round-trips
    /// happily on its own). Keep it last; never construct it, never emit it —
    /// it is absent from `ALL`, so `kind_from_raw` reads it back as `Error`.
    #[doc(hidden)]
    __NotAKind,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

impl SyntaxKind {
    /// Whether this kind is trivia — present in the tree for fidelity, skipped
    /// by every grammar rule.
    ///
    /// The set is owned by [`Token::is_trivia`]; this is its image under
    /// `From<Token>`, and `trivia_agrees_with_the_lexer` proves the two stay in
    /// step in both directions. The duplication is unavoidable — the tree is
    /// built from `SyntaxKind`, not `Token` — but the *drift* is not.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            SyntaxKind::Whitespace
                | SyntaxKind::Newline
                | SyntaxKind::LineComment
                | SyntaxKind::BlockComment
        )
    }

    /// Whether this kind is a type node — something `ast_type_from_cst` can
    /// turn into an `AstType`.
    ///
    /// Excludes `TyList`, which *contains* types (a function parameter list)
    /// but is not one. `every_ty_kind_is_a_type_node` proves the set covers
    /// every `Ty*` variant except that one, so appending a type kind to the
    /// enum without listing it here fails the build rather than making the
    /// new syntax invisible to the AST builder.
    pub fn is_type(self) -> bool {
        matches!(
            self,
            SyntaxKind::Ty
                | SyntaxKind::TyPath
                | SyntaxKind::TyTuple
                | SyntaxKind::TyFunction
                | SyntaxKind::TyArray
                | SyntaxKind::TyDictionary
                | SyntaxKind::TyOptional
                | SyntaxKind::TyResult
                | SyntaxKind::TyUnit
                | SyntaxKind::TyNever
                | SyntaxKind::TyInferred
                | SyntaxKind::TySome
                | SyntaxKind::TyRef
                | SyntaxKind::TyMutRef
        )
    }

    /// `Ty*` kinds that are deliberately **not** type nodes. Each needs a
    /// reason — this list is the only way past `every_ty_kind_is_a_type_node`.
    #[cfg(test)]
    const NON_TYPE_TY_KINDS: &'static [(SyntaxKind, &'static str)] = &[(
        SyntaxKind::TyList,
        "a list of parameter types, not a type itself",
    )];

    /// Every variant, **in declaration order** — `ALL[n]` is the kind whose
    /// discriminant is `n`. `SyntaxKind` declares no explicit discriminants, so
    /// declaration order *is* the raw numbering rowan stores in green trees.
    ///
    /// This is the inverse of `kind as u16`, and the only reason it can be
    /// hand-written is that `syntax_kind_table_round_trips` proves it complete
    /// and correctly ordered. **Append a new kind at the end of both the enum
    /// and this table** — inserting mid-list renumbers every later kind and
    /// corrupts cached trees.
    pub const ALL: &'static [SyntaxKind] = &[
        SyntaxKind::Root,
        SyntaxKind::SourceFile,
        SyntaxKind::DeclarationItem,
        SyntaxKind::Attribute,
        SyntaxKind::AttributeList,
        SyntaxKind::AttributeArgs,
        SyntaxKind::AttributeArg,
        SyntaxKind::ProtocolDeclaration,
        SyntaxKind::ProtocolBody,
        SyntaxKind::StructDeclaration,
        SyntaxKind::StructBody,
        SyntaxKind::ExtensionDeclaration,
        SyntaxKind::ExtensionBody,
        SyntaxKind::EnumDeclaration,
        SyntaxKind::EnumBody,
        SyntaxKind::EnumCaseDeclaration,
        SyntaxKind::EnumCaseParameter,
        SyntaxKind::EnumCaseParameterList,
        SyntaxKind::IndirectModifier,
        SyntaxKind::ImportDeclaration,
        SyntaxKind::ImportItem,
        SyntaxKind::ModuleDeclaration,
        SyntaxKind::ModulePath,
        SyntaxKind::Name,
        SyntaxKind::TypeAliasDeclaration,
        SyntaxKind::AliasedType,
        SyntaxKind::FieldDeclaration,
        SyntaxKind::GetterClause,
        SyntaxKind::SetterClause,
        SyntaxKind::PropertyAccessors,
        SyntaxKind::FunctionDeclaration,
        SyntaxKind::InitializerDeclaration,
        SyntaxKind::InitEffect,
        SyntaxKind::DeinitDeclaration,
        SyntaxKind::SubscriptDeclaration,
        SyntaxKind::SubscriptBody,
        SyntaxKind::FunctionBody,
        SyntaxKind::ParameterList,
        SyntaxKind::Parameter,
        SyntaxKind::ReturnType,
        SyntaxKind::Visibility,
        SyntaxKind::StaticModifier,
        SyntaxKind::TypeParameterList,
        SyntaxKind::TypeParameter,
        SyntaxKind::TypeArgumentList,
        SyntaxKind::DefaultType,
        SyntaxKind::DefaultValue,
        SyntaxKind::WhereClause,
        SyntaxKind::TypeBound,
        SyntaxKind::TypeEquality,
        SyntaxKind::AssociatedTypeBound,
        SyntaxKind::AssociatedTypeTarget,
        SyntaxKind::ConformanceList,
        SyntaxKind::ConformanceItem,
        SyntaxKind::NegativeConformance,
        SyntaxKind::Ty,
        SyntaxKind::TyUnit,
        SyntaxKind::TyNever,
        SyntaxKind::TyTuple,
        SyntaxKind::TyFunction,
        SyntaxKind::TyPath,
        SyntaxKind::TyArray,
        SyntaxKind::TyDictionary,
        SyntaxKind::TyOptional,
        SyntaxKind::TyResult,
        SyntaxKind::TyList,
        SyntaxKind::TyInferred,
        SyntaxKind::TySome,
        SyntaxKind::Path,
        SyntaxKind::PathElement,
        SyntaxKind::CodeBlock,
        SyntaxKind::Statement,
        SyntaxKind::ExpressionStatement,
        SyntaxKind::VariableDeclaration,
        SyntaxKind::GuardStatement,
        SyntaxKind::DeinitStatement,
        SyntaxKind::GuardCondition,
        SyntaxKind::Expression,
        SyntaxKind::ExprUnit,
        SyntaxKind::ExprInteger,
        SyntaxKind::ExprFloat,
        SyntaxKind::ExprString,
        SyntaxKind::ExprRawString,
        SyntaxKind::ExprInterpolatedString,
        SyntaxKind::StringLiteralPart,
        SyntaxKind::StringInterpolation,
        SyntaxKind::FormatSpecifier,
        SyntaxKind::ExprChar,
        SyntaxKind::ExprBool,
        SyntaxKind::ExprArray,
        SyntaxKind::ExprDictionary,
        SyntaxKind::DictionaryEntry,
        SyntaxKind::ExprTuple,
        SyntaxKind::ExprGrouping,
        SyntaxKind::ExprPath,
        SyntaxKind::ExprUnary,
        SyntaxKind::ExprPostfix,
        SyntaxKind::ExprBinary,
        SyntaxKind::ExprNull,
        SyntaxKind::ExprCall,
        SyntaxKind::ExprAssignment,
        SyntaxKind::ExprCompoundAssignment,
        SyntaxKind::ExprIf,
        SyntaxKind::IfLetCondition,
        SyntaxKind::ElseClause,
        SyntaxKind::ExprWhile,
        SyntaxKind::WhileLetCondition,
        SyntaxKind::ExprFor,
        SyntaxKind::ForPattern,
        SyntaxKind::ForIterable,
        SyntaxKind::ExprLoop,
        SyntaxKind::ExprBreak,
        SyntaxKind::ExprContinue,
        SyntaxKind::ExprReturn,
        SyntaxKind::ExprThrow,
        SyntaxKind::ExprTry,
        SyntaxKind::ExprTupleIndex,
        SyntaxKind::ExprClosure,
        SyntaxKind::ClosureParams,
        SyntaxKind::ClosureParam,
        SyntaxKind::LoopLabel,
        SyntaxKind::ArgumentList,
        SyntaxKind::Argument,
        SyntaxKind::ExprImplicitMemberAccess,
        SyntaxKind::ExprMatch,
        SyntaxKind::MatchArm,
        SyntaxKind::MatchArmGuard,
        SyntaxKind::Pattern,
        SyntaxKind::WildcardPattern,
        SyntaxKind::BindingPattern,
        SyntaxKind::TuplePattern,
        SyntaxKind::TuplePatternElement,
        SyntaxKind::LiteralPattern,
        SyntaxKind::RangePattern,
        SyntaxKind::EnumPattern,
        SyntaxKind::EnumPatternArg,
        SyntaxKind::NullPattern,
        SyntaxKind::SomePattern,
        SyntaxKind::StructPattern,
        SyntaxKind::StructPatternField,
        SyntaxKind::StructPatternRest,
        SyntaxKind::ArrayPattern,
        SyntaxKind::ArrayPatternElement,
        SyntaxKind::ArrayPatternRest,
        SyntaxKind::AtPattern,
        SyntaxKind::RestPattern,
        SyntaxKind::OrPattern,
        SyntaxKind::ErrorPattern,
        SyntaxKind::Identifier,
        SyntaxKind::String,
        SyntaxKind::RawString,
        SyntaxKind::Char,
        SyntaxKind::Integer,
        SyntaxKind::Float,
        SyntaxKind::Boolean,
        SyntaxKind::Null,
        SyntaxKind::Some,
        SyntaxKind::As,
        SyntaxKind::Break,
        SyntaxKind::Case,
        SyntaxKind::Consuming,
        SyntaxKind::Continue,
        SyntaxKind::Deinit,
        SyntaxKind::Else,
        SyntaxKind::Enum,
        SyntaxKind::Extend,
        SyntaxKind::For,
        SyntaxKind::Fileprivate,
        SyntaxKind::Func,
        SyntaxKind::If,
        SyntaxKind::Import,
        SyntaxKind::Indirect,
        SyntaxKind::Loop,
        SyntaxKind::Init,
        SyntaxKind::Internal,
        SyntaxKind::Let,
        SyntaxKind::Module,
        SyntaxKind::Mutating,
        SyntaxKind::Private,
        SyntaxKind::Protocol,
        SyntaxKind::Public,
        SyntaxKind::Return,
        SyntaxKind::Throw,
        SyntaxKind::Try,
        SyntaxKind::Throws,
        SyntaxKind::Static,
        SyntaxKind::Struct,
        SyntaxKind::Type,
        SyntaxKind::Var,
        SyntaxKind::Where,
        SyntaxKind::While,
        SyntaxKind::In,
        SyntaxKind::Match,
        SyntaxKind::Guard,
        SyntaxKind::Get,
        SyntaxKind::Set,
        SyntaxKind::Subscript,
        SyntaxKind::And,
        SyntaxKind::Not,
        SyntaxKind::Or,
        SyntaxKind::LParen,
        SyntaxKind::RParen,
        SyntaxKind::LBrace,
        SyntaxKind::RBrace,
        SyntaxKind::LBracket,
        SyntaxKind::RBracket,
        SyntaxKind::Semicolon,
        SyntaxKind::Comma,
        SyntaxKind::Dot,
        SyntaxKind::Colon,
        SyntaxKind::Question,
        SyntaxKind::Bang,
        SyntaxKind::Underscore,
        SyntaxKind::DotDotEquals,
        SyntaxKind::DotDotLess,
        SyntaxKind::DotDot,
        SyntaxKind::LessLessEquals,
        SyntaxKind::GreaterGreaterEquals,
        SyntaxKind::LessLess,
        SyntaxKind::GreaterGreater,
        SyntaxKind::LessEquals,
        SyntaxKind::GreaterEquals,
        SyntaxKind::EqualsEquals,
        SyntaxKind::BangEquals,
        SyntaxKind::QuestionQuestion,
        SyntaxKind::Arrow,
        SyntaxKind::FatArrow,
        SyntaxKind::PlusEquals,
        SyntaxKind::MinusEquals,
        SyntaxKind::StarEquals,
        SyntaxKind::SlashEquals,
        SyntaxKind::PercentEquals,
        SyntaxKind::AmpersandEquals,
        SyntaxKind::PipeEquals,
        SyntaxKind::CaretEquals,
        SyntaxKind::Equals,
        SyntaxKind::Plus,
        SyntaxKind::Minus,
        SyntaxKind::Star,
        SyntaxKind::Slash,
        SyntaxKind::Percent,
        SyntaxKind::Ampersand,
        SyntaxKind::Pipe,
        SyntaxKind::Caret,
        SyntaxKind::Less,
        SyntaxKind::Greater,
        SyntaxKind::At,
        SyntaxKind::Whitespace,
        SyntaxKind::Newline,
        SyntaxKind::LineComment,
        SyntaxKind::BlockComment,
        SyntaxKind::Error,
        SyntaxKind::Missing,
        SyntaxKind::TyRef,
        SyntaxKind::TyMutRef,
        SyntaxKind::RefClause,
        SyntaxKind::MutatingRefClause,
        SyntaxKind::RefBindingPattern,
        SyntaxKind::StringStart,
        SyntaxKind::StringFragment,
        SyntaxKind::InterpStart,
        SyntaxKind::InterpEnd,
        SyntaxKind::FormatSpec,
        SyntaxKind::StringEnd,
    ];
}

impl From<Token> for SyntaxKind {
    fn from(token: Token) -> Self {
        match token {
            // Trivia
            Token::Whitespace => SyntaxKind::Whitespace,
            Token::Newline => SyntaxKind::Newline,
            Token::LineComment => SyntaxKind::LineComment,
            Token::BlockComment => SyntaxKind::BlockComment,
            // Literals
            Token::Identifier => SyntaxKind::Identifier,
            Token::String => SyntaxKind::String,
            Token::StringStart => SyntaxKind::StringStart,
            Token::StringFragment => SyntaxKind::StringFragment,
            Token::InterpStart => SyntaxKind::InterpStart,
            Token::InterpEnd => SyntaxKind::InterpEnd,
            Token::FormatSpec => SyntaxKind::FormatSpec,
            Token::StringEnd => SyntaxKind::StringEnd,
            Token::RawString => SyntaxKind::RawString,
            Token::Char => SyntaxKind::Char,
            Token::Integer => SyntaxKind::Integer,
            Token::Float => SyntaxKind::Float,
            Token::Boolean => SyntaxKind::Boolean,
            Token::Null => SyntaxKind::Null,
            Token::Some => SyntaxKind::Some,
            // Keywords
            Token::As => SyntaxKind::As,
            Token::Break => SyntaxKind::Break,
            Token::Case => SyntaxKind::Case,
            Token::Consuming => SyntaxKind::Consuming,
            Token::Continue => SyntaxKind::Continue,
            Token::Deinit => SyntaxKind::Deinit,
            Token::Else => SyntaxKind::Else,
            Token::Enum => SyntaxKind::Enum,
            Token::Extend => SyntaxKind::Extend,
            Token::For => SyntaxKind::For,
            Token::Fileprivate => SyntaxKind::Fileprivate,
            Token::Func => SyntaxKind::Func,
            Token::If => SyntaxKind::If,
            Token::Import => SyntaxKind::Import,
            Token::Indirect => SyntaxKind::Indirect,
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
            Token::Throw => SyntaxKind::Throw,
            Token::Try => SyntaxKind::Try,
            Token::Throws => SyntaxKind::Throws,
            Token::Static => SyntaxKind::Static,
            Token::Struct => SyntaxKind::Struct,
            Token::Type => SyntaxKind::Type,
            Token::Var => SyntaxKind::Var,
            Token::Where => SyntaxKind::Where,
            Token::While => SyntaxKind::While,
            Token::In => SyntaxKind::In,
            Token::Match => SyntaxKind::Match,
            Token::Guard => SyntaxKind::Guard,
            Token::Get => SyntaxKind::Get,
            Token::Set => SyntaxKind::Set,
            Token::Subscript => SyntaxKind::Subscript,
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
            Token::DotDot => SyntaxKind::DotDot,
            Token::LessLessEquals => SyntaxKind::LessLessEquals,
            Token::GreaterGreaterEquals => SyntaxKind::GreaterGreaterEquals,
            Token::LessLess => SyntaxKind::LessLess,
            Token::GreaterGreater => SyntaxKind::GreaterGreater,
            Token::LessEquals => SyntaxKind::LessEquals,
            Token::GreaterEquals => SyntaxKind::GreaterEquals,
            Token::EqualsEquals => SyntaxKind::EqualsEquals,
            Token::BangEquals => SyntaxKind::BangEquals,
            Token::QuestionQuestion => SyntaxKind::QuestionQuestion,
            Token::Arrow => SyntaxKind::Arrow,
            Token::FatArrow => SyntaxKind::FatArrow,
            Token::PlusEquals => SyntaxKind::PlusEquals,
            Token::MinusEquals => SyntaxKind::MinusEquals,
            Token::StarEquals => SyntaxKind::StarEquals,
            Token::SlashEquals => SyntaxKind::SlashEquals,
            Token::PercentEquals => SyntaxKind::PercentEquals,
            Token::AmpersandEquals => SyntaxKind::AmpersandEquals,
            Token::PipeEquals => SyntaxKind::PipeEquals,
            Token::CaretEquals => SyntaxKind::CaretEquals,
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
            Token::At => SyntaxKind::At,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KestrelLanguage;

impl Language for KestrelLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        // `kind_to_raw` is `kind as u16`, so the inverse is a plain index into
        // the declaration-order table. This used to be 258 hand-written
        // `const NAME: u16 = SyntaxKind::Name as u16;` declarations plus 258
        // hand-written match arms over `raw.0` — and because the scrutinee was
        // a `u16`, rustc could not check either list. A kind appended to the
        // enum without a matching arm silently read back as `Error`, which is
        // the *recovery* marker, so the tree would look damaged rather than
        // unknown (F27). `syntax_kind_table_round_trips` proves `ALL` is
        // complete and in order.
        SyntaxKind::ALL
            .get(raw.0 as usize)
            .copied()
            .unwrap_or(SyntaxKind::Error)
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

    /// `SyntaxKind::ALL` is the hand-written inverse of `kind as u16`. Three
    /// properties make it safe to hand-write; this test is all three.
    ///
    /// 1. **Ordered** — `ALL[n] as u16 == n`, so indexing by a raw value is the
    ///    correct inverse.
    /// 2. **Complete** — every kind round-trips through rowan's raw form. A
    ///    kind appended to the enum but not to `ALL` fails here instead of
    ///    silently reading back as `Error`, which is the *recovery* marker: the
    ///    tree would look damaged rather than unknown (F27).
    /// 3. **Total** — the entry for `Error` itself round-trips, so the
    ///    out-of-range fallback is not masking a real kind.
    #[test]
    fn syntax_kind_table_round_trips() {
        for (index, &kind) in SyntaxKind::ALL.iter().enumerate() {
            assert_eq!(
                kind as usize, index,
                "SyntaxKind::ALL[{index}] is {kind:?}, whose discriminant is {}. \
                 The table must be in declaration order — a kind was inserted \
                 mid-list instead of appended.",
                kind as usize
            );
            let raw = KestrelLanguage::kind_to_raw(kind);
            assert_eq!(
                KestrelLanguage::kind_from_raw(raw),
                kind,
                "{kind:?} does not round-trip through rowan's raw form"
            );
        }
        // Completeness: `__NotAKind` sits immediately after the last real
        // variant, so its discriminant IS the count. Without this, a table
        // missing its final entries still round-trips — every entry it *does*
        // hold is correct, and the missing kinds are simply never tested.
        assert_eq!(
            SyntaxKind::__NotAKind as usize,
            SyntaxKind::ALL.len(),
            "SyntaxKind::ALL is missing {} kind(s) — append the new variant(s) \
             to the table too",
            SyntaxKind::__NotAKind as usize - SyntaxKind::ALL.len()
        );

        // Anything past the table is genuinely unknown and must read as Error.
        let past_end = rowan::SyntaxKind(SyntaxKind::ALL.len() as u16);
        assert_eq!(
            KestrelLanguage::kind_from_raw(past_end),
            SyntaxKind::Error
        );
    }

    /// The type-node set is derivable from the enum itself: a `Ty*` variant is
    /// a type node unless it is explicitly excused. This is the check that was
    /// missing when `is_type_node` (14 variants) and `is_type_kind` (12) drifted
    /// apart — `TyRef`/`TyMutRef` were appended to only one of them, and the
    /// omission was masked only by the parser wrapping `TyRef` inside a `Ty`.
    #[test]
    fn every_ty_kind_is_a_type_node() {
        for &kind in SyntaxKind::ALL {
            let name = format!("{kind:?}");
            // `Type*` (TypeBound, TypeParameter, …) are not type *nodes*.
            if !name.starts_with("Ty") || name.starts_with("Type") {
                assert!(
                    !kind.is_type(),
                    "{kind:?} is marked a type node but is not a `Ty*` kind"
                );
                continue;
            }
            let excused = SyntaxKind::NON_TYPE_TY_KINDS
                .iter()
                .find(|(k, _)| *k == kind);
            match excused {
                Some((_, reason)) => assert!(
                    !kind.is_type(),
                    "{kind:?} is excused from being a type node ({reason}) \
                     but `is_type` claims it is one"
                ),
                None => assert!(
                    kind.is_type(),
                    "{kind:?} is a `Ty*` kind but `SyntaxKind::is_type` does not \
                     list it. Add it, or add it to NON_TYPE_TY_KINDS with a reason."
                ),
            }
        }
    }

    /// The trivia set is defined once, on `Token`. `SyntaxKind::is_trivia` is
    /// its image under `From<Token>`, and the two must not drift: a token the
    /// grammar skips whose kind is not marked trivia breaks CST navigation,
    /// and a kind marked trivia with no trivia token behind it can never match.
    #[test]
    fn trivia_agrees_with_the_lexer() {
        // Forward: every trivia token's kind is trivia, and no other token's is.
        let trivia_tokens = [
            Token::Whitespace,
            Token::Newline,
            Token::LineComment,
            Token::BlockComment,
        ];
        for token in &trivia_tokens {
            assert!(token.is_trivia(), "{token:?} must be trivia");
            let kind = SyntaxKind::from(token.clone());
            assert!(
                kind.is_trivia(),
                "{token:?} is trivia but SyntaxKind::{kind:?} is not"
            );
        }
        for token in [Token::Identifier, Token::Func, Token::LBrace, Token::String] {
            assert!(!token.is_trivia());
            assert!(!SyntaxKind::from(token).is_trivia());
        }

        // Backward, and the half that actually catches drift: no kind may be
        // marked trivia without a trivia token behind it. Splitting `///` out
        // of `LineComment` adds a fifth trivia kind and fails here, which is
        // the signal to add the matching `Token` arm rather than teach one
        // call site about the new kind.
        let trivia_kinds: Vec<_> = SyntaxKind::ALL
            .iter()
            .copied()
            .filter(|k| k.is_trivia())
            .collect();
        let expected: Vec<_> = trivia_tokens
            .iter()
            .cloned()
            .map(SyntaxKind::from)
            .collect();
        assert_eq!(
            trivia_kinds, expected,
            "SyntaxKind's trivia set drifted from Token's — the set is owned by \
             Token::is_trivia; update it there and map the new token"
        );

        // `is_inline_trivia` is a strict subset differing only in `Newline`.
        for token in &trivia_tokens {
            assert_eq!(
                token.is_inline_trivia(),
                token.is_trivia() && *token != Token::Newline
            );
        }
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
