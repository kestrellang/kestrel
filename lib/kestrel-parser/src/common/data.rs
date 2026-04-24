//! Shared data structures used across multiple parser modules
//!
//! This module contains data types that are used by multiple parsers
//! to avoid duplication and ensure consistency.

use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::block::CodeBlockData;
use crate::expr::ExprVariant;
use crate::field::FieldDeclarationData;
use crate::function::FunctionDeclarationData;
use crate::pattern::PatternVariant;
use crate::subscript::SubscriptDeclarationData;
use crate::ty::TyVariant;
use crate::type_alias::TypeAliasDeclarationData;
use crate::type_param::{TypeParameterData, WhereClauseData};

// =============================================================================
// Attribute Data Structures
// =============================================================================

/// Value types that can appear in attribute arguments
#[derive(Debug, Clone)]
pub enum AttributeArgValue {
    /// String literal: `"value"`
    String(Span),
    /// Integer literal: `42`
    Integer(Span),
    /// Float literal: `3.14`
    Float(Span),
    /// Boolean literal: `true` or `false`
    Bool(Span),
    /// Implicit member access: `.option`
    ImplicitMember { dot_span: Span, name_span: Span },
    /// Path: `SomeType` or `Module.Type`
    Path(Vec<Span>), // segments (identifiers only, dots are implicit between them)
}

/// Raw parsed data for a single attribute argument
#[derive(Debug, Clone)]
pub struct AttributeArgData {
    /// Optional label (e.g., `iOS` in `iOS: 15.0`)
    pub label: Option<Span>,
    /// Optional colon after label
    pub colon: Option<Span>,
    /// The value expression
    pub value: AttributeArgValue,
}

/// Raw parsed data for attribute arguments (the contents of the parentheses)
#[derive(Debug, Clone)]
pub struct AttributeArgsData {
    pub lparen_span: Span,
    pub args: Vec<AttributeArgData>,
    pub rparen_span: Span,
}

/// Raw parsed data for a single attribute
#[derive(Debug, Clone)]
pub struct AttributeData {
    /// The @ token span
    pub at_span: Span,
    /// The attribute name span
    pub name_span: Span,
    /// Optional arguments in parentheses
    pub args: Option<AttributeArgsData>,
}

/// Access mode for function parameters.
///
/// Determines how the caller's value is passed and what the callee can do with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterAccessMode {
    /// Read-only access (default). Caller retains ownership.
    /// Syntax: `x: T`
    Borrow,
    /// Read-write access. Caller retains ownership but must use `var` binding.
    /// Syntax: `mutating x: T`
    Mutating,
    /// Takes ownership (move or copy depending on Copyable).
    /// Syntax: `consuming x: T`
    Consuming,
}

/// Raw parsed data for a single parameter
///
/// Parameter syntax: `(access_mode)? (label)? pattern: Type (= default)?`
/// - `access_mode` is an optional access mode (mutating/consuming)
/// - `label` is an optional external parameter name (used by callers)
/// - `pattern` is the binding pattern (identifier, tuple, struct, or wildcard)
/// - If only one identifier is provided (no label), it's used as both label and pattern
/// - `default` is an optional default value expression
///
/// # Examples
/// - `x: Int` → access_mode=None, label=None, pattern=Binding(x)
/// - `with x: Int` → access_mode=None, label="with", pattern=Binding(x)
/// - `mutating x: Int` → access_mode=Mutating, label=None, pattern=Binding(x)
/// - `(a, b): (Int, Int)` → access_mode=None, label=None, pattern=Tuple
/// - `point (x, y): Point` → access_mode=None, label="point", pattern=Tuple
/// - `Point { x, y }: Point` → access_mode=None, label=None, pattern=Struct
/// - `_: Int` → access_mode=None, label=None, pattern=Wildcard
/// - `x: Int = 0` → access_mode=None, label=None, pattern=Binding(x), default=Some(0)
#[derive(Debug, Clone)]
pub struct ParameterData {
    /// Optional access mode (mutating/consuming)
    /// If None, the default is borrow (read-only)
    pub access_mode: Option<(ParameterAccessMode, Span)>,
    /// Optional label (external name for callers)
    /// If None, the pattern's primary name is used as the label (for binding patterns)
    pub label: Option<Span>,
    /// The binding pattern (identifier, tuple, struct, or wildcard)
    pub pattern: PatternVariant,
    /// The colon span
    pub colon: Span,
    /// The parameter type
    pub ty: TyVariant,
    /// Optional default value (equals_span, expression)
    pub default: Option<(Span, ExprVariant)>,
}

/// Body data for functions - either a block `{ ... }` or expression `= expr`
#[derive(Debug, Clone)]
pub enum FunctionBodyData {
    /// Block body: `{ statements; expr }`
    Block(CodeBlockData),
    /// Expression body: `= expr`
    /// Contains the equals span and the expression
    Expression(Span, ExprVariant),
}

/// Raw parsed data for initializer declaration internals
///
/// Initializer syntax: `(visibility)? init[T]?(params) where ...? { body }?`
/// Body is optional for protocol initializer declarations.
#[derive(Debug, Clone)]
pub struct InitializerDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub init_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub lparen: Span,
    pub parameters: Vec<ParameterData>,
    pub rparen: Span,
    pub where_clause: Option<WhereClauseData>,
    pub body: Option<CodeBlockData>,
}

/// Raw parsed data for deinitializer declaration internals
///
/// Deinit syntax: `deinit { body }`
/// The body is required. Deinit blocks have no parameters or visibility.
/// Deinit runs when a value goes out of scope to clean up resources.
#[derive(Debug, Clone)]
pub struct DeinitDeclarationData {
    pub deinit_span: Span,
    pub body: CodeBlockData,
}

/// A single conformance item, which can be positive or negative
#[derive(Debug, Clone)]
pub struct ConformanceItemData {
    /// If Some, this is a negative conformance (e.g., `not Copyable`)
    pub not_span: Option<Span>,
    /// The protocol type
    pub ty: TyVariant,
}

/// Raw parsed data for a conformance list (: Proto1, Proto2, not Copyable)
#[derive(Debug, Clone)]
pub struct ConformanceListData {
    pub colon_span: Span,
    pub conformances: Vec<ConformanceItemData>,
}

/// Raw parsed data for struct declaration internals
#[derive(Debug, Clone)]
pub struct StructDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub struct_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub conformances: Option<ConformanceListData>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<TypeDeclarationBodyItem>,
    pub rbrace_span: Span,
}

/// Items that can appear in a type declaration body (struct or enum)
/// Used to enable mutual nesting of structs and enums
#[derive(Debug, Clone)]
pub enum TypeDeclarationBodyItem {
    Field(FieldDeclarationData),
    Function(FunctionDeclarationData),
    Subscript(SubscriptDeclarationData),
    Initializer(InitializerDeclarationData),
    Deinit(DeinitDeclarationData), // deinit { } - only valid in struct bodies
    Struct(Box<StructDeclarationData>), // Boxed to avoid infinite size
    Enum(Box<EnumDeclarationData>), // Boxed to avoid infinite size
    EnumCase(EnumCaseDeclarationData), // Only valid in enum bodies
    TypeAlias(TypeAliasDeclarationData), // Associated type bindings
    Module(Span, Vec<Span>),       // module_span, path_segments
    Import(
        Span,
        Vec<Span>,
        Option<Span>,
        Option<Vec<(Span, Option<Span>)>>,
    ), // import_span, path, alias, items
}

/// Deprecated: Use TypeDeclarationBodyItem instead
/// Kept for backwards compatibility during migration
#[deprecated(note = "Use TypeDeclarationBodyItem instead")]
pub type StructBodyItem = TypeDeclarationBodyItem;

/// Raw parsed data for enum case parameter
///
/// Supports both named (`label: Type`) and unnamed (`Type`) forms:
/// - Named: `case Some(value: T)` - label and colon present
/// - Unnamed: `case Some(T)` - label and colon are None
#[derive(Debug, Clone)]
pub struct EnumCaseParameterData {
    /// Optional label name (None for unnamed parameters)
    pub label: Option<Span>,
    /// Optional colon (present only when label is present)
    pub colon: Option<Span>,
    /// The type of the parameter
    pub ty: TyVariant,
}

/// Raw parsed data for enum case declaration
#[derive(Debug, Clone)]
pub struct EnumCaseDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub case_span: Span,
    pub name_span: Span,
    pub parameters: Option<(Span, Vec<EnumCaseParameterData>, Span)>, // (lparen, params, rparen)
}

/// Raw parsed data for enum declaration
#[derive(Debug, Clone)]
pub struct EnumDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub indirect: Option<Span>,
    pub enum_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub conformances: Option<ConformanceListData>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<TypeDeclarationBodyItem>,
    pub rbrace_span: Span,
}

/// Raw parsed data for protocol declaration internals
#[derive(Debug, Clone)]
pub struct ProtocolDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub protocol_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub inherited: Option<ConformanceListData>, // Inherited protocols (protocol A: B { })
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<ProtocolBodyItem>, // Protocol body: functions and associated types
    pub rbrace_span: Span,
}

/// Items that can appear in a protocol body
#[derive(Debug, Clone)]
pub enum ProtocolBodyItem {
    Function(FunctionDeclarationData),
    Subscript(SubscriptDeclarationData),
    AssociatedType(TypeAliasDeclarationData),
    Initializer(InitializerDeclarationData),
    Field(FieldDeclarationData),
}

/// Raw parsed data for extension declaration internals
///
/// Extension syntax: `extend Type: Protocol { ... }`
/// Extensions add methods and conformances to existing types.
#[derive(Debug, Clone)]
pub struct ExtensionDeclarationData {
    pub extend_span: Span,
    /// The type being extended (uses type expression, not type parameter list)
    /// This allows Box[T, Int] where T references the struct's type parameter
    pub target_type: TyVariant,
    /// Optional conformances this extension adds
    pub conformances: Option<ConformanceListData>,
    /// Optional where clause for additional constraints
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<ExtensionBodyItem>,
    pub rbrace_span: Span,
}

/// Items that can appear in an extension body
#[derive(Debug, Clone)]
pub enum ExtensionBodyItem {
    Function(FunctionDeclarationData),
    Subscript(SubscriptDeclarationData),
    Initializer(InitializerDeclarationData),
    TypeAlias(TypeAliasDeclarationData),
}

