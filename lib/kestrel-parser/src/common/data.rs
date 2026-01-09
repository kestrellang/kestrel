//! Shared data structures used across multiple parser modules
//!
//! This module contains data types that are used by multiple parsers
//! to avoid duplication and ensure consistency.

use kestrel_lexer::Token;
use kestrel_span::Span;

use crate::block::CodeBlockData;
use crate::ty::TyVariant;
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
/// Parameter syntax: `(access_mode)? (label)? bind_name: Type`
/// - `access_mode` is an optional access mode (mutating/consuming)
/// - `label` is an optional external parameter name (used by callers)
/// - `bind_name` is the internal parameter name (used in function body)
/// - If only one name is provided, it's used as both label and bind_name
///
/// # Examples
/// - `x: Int` → access_mode=None, label=None, bind_name=x
/// - `with x: Int` → access_mode=None, label="with", bind_name=x
/// - `mutating x: Int` → access_mode=Mutating, label=None, bind_name=x
/// - `consuming point p: Point` → access_mode=Consuming, label="point", bind_name=p
#[derive(Debug, Clone)]
pub struct ParameterData {
    /// Optional access mode (mutating/consuming)
    /// If None, the default is borrow (read-only)
    pub access_mode: Option<(ParameterAccessMode, Span)>,
    /// Optional label (external name for callers)
    /// If None, bind_name is used as the label
    pub label: Option<Span>,
    /// The binding name (internal name used in function body)
    pub bind_name: Span,
    /// The colon span
    pub colon: Span,
    /// The parameter type
    pub ty: TyVariant,
}

/// Receiver modifier for instance methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverModifier {
    /// `mutating func` - method can mutate self
    Mutating,
    /// `consuming func` - method takes ownership of self
    Consuming,
}

/// Raw parsed data for function declaration internals
///
/// Used by both function declarations and protocol method declarations.
#[derive(Debug, Clone)]
pub struct FunctionDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub is_static: Option<Span>,
    /// Receiver modifier (mutating/consuming) with its span
    pub receiver_modifier: Option<(ReceiverModifier, Span)>,
    pub fn_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub lparen: Span,
    pub parameters: Vec<ParameterData>,
    pub rparen: Span,
    pub return_type: Option<(Span, TyVariant)>, // (arrow_span, return_ty)
    pub where_clause: Option<WhereClauseData>,
    pub body: Option<CodeBlockData>, // Optional code block - None for protocol methods
}

/// Raw parsed data for field declaration internals
#[derive(Debug, Clone)]
pub struct FieldDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub is_static: Option<Span>,
    pub mutability_span: Span,
    pub is_mutable: bool,
    pub name_span: Span,
    pub colon_span: Span,
    pub ty: TyVariant,
    /// Optional trailing semicolon (for inline field declarations)
    pub semicolon: Option<Span>,
}

/// Raw parsed data for initializer declaration internals
///
/// Initializer syntax: `(visibility)? init(params) { body }?`
/// Body is optional for protocol initializer declarations.
#[derive(Debug, Clone)]
pub struct InitializerDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub init_span: Span,
    pub lparen: Span,
    pub parameters: Vec<ParameterData>,
    pub rparen: Span,
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

/// Raw parsed data for type alias declaration internals
#[derive(Debug, Clone)]
pub struct TypeAliasDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub type_span: Span,
    /// The target of the type alias - simple name or qualified path
    pub target: AssociatedTypeTargetData,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    /// Optional bounds for associated types (: Equatable, Hashable)
    pub bounds: Option<AssociatedTypeBoundsData>,
    /// Optional equals span and aliased type (= Type)
    /// For associated types in protocols, this may be None (abstract associated type)
    pub aliased: Option<(Span, TyVariant)>,
    pub semicolon_span: Option<Span>,
}

/// Target for type alias - either simple name or qualified path
#[derive(Debug, Clone)]
pub enum AssociatedTypeTargetData {
    /// Simple name: `type Item`
    Simple(Span),
    /// Qualified path: `type Iterator.Item` or `type Add[Int].Output`
    Qualified {
        /// The protocol path (may include type arguments)
        protocol_path: TyVariant,
        /// The dot before the name
        dot_span: Span,
        /// The associated type name
        name_span: Span,
    },
}

/// Bounds for associated types (: Equatable, Hashable)
#[derive(Debug, Clone)]
pub struct AssociatedTypeBoundsData {
    pub colon_span: Span,
    /// The bound types (protocols)
    pub bounds: Vec<TyVariant>,
}

/// Items that can appear in a protocol body
#[derive(Debug, Clone)]
pub enum ProtocolBodyItem {
    Function(FunctionDeclarationData),
    AssociatedType(TypeAliasDeclarationData),
    Initializer(InitializerDeclarationData),
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
    Initializer(InitializerDeclarationData),
}
