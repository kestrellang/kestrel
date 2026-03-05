//! All component types for declaration entities in the ECS world.
//!
//! Components describe capabilities — what an entity CAN DO. They are
//! orthogonal and composable, derived entirely from the CST during the
//! build (mutation) phase.

use kestrel_hecs::Entity;
use kestrel_span2::Span;
use kestrel_syntax_tree2::SyntaxNode;

use crate::ast_type::AstType;

// ===== Identity (on every declaration entity) =====

/// What kind of declaration this entity represents.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Module,
    Struct,
    Enum,
    EnumCase,
    Protocol,
    Extension,
    Function,
    Initializer,
    Deinit,
    Field,
    Subscript,
    TypeAlias,
    Import,
    TypeParameter,
}

/// Source span excluding leading trivia.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeclSpan(pub Span);

/// Cheap Arc-backed CST reference for this declaration.
#[derive(Clone, Debug)]
pub struct CstNode(pub SyntaxNode);

// ===== Naming & location =====

/// Declared identifier name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name(pub String);

/// Source file entity this declaration belongs to.
/// Modules don't get FileId — they span multiple files.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub Entity);

/// Visibility modifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Vis {
    Public,
    Private,
    Internal,
    Fileprivate,
}

// ===== Capability components (orthogonal axes) =====

/// Marker: this entity IS a type (can appear in type positions).
/// Applied to Struct, Enum, Protocol, TypeAlias.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Typed;

/// Has a type annotation (field type, return type, alias target).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeAnnotation(pub AstType);

/// Has a parameter list, can be invoked.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Callable {
    pub params: Vec<AstParam>,
    pub receiver: Option<ReceiverKind>,
}

/// A single parameter in a callable signature.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AstParam {
    pub label: Option<String>,
    pub name: String,
    pub ty: Option<AstType>,
    pub has_default: bool,
}

/// How a method receives its self argument.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReceiverKind {
    Borrowing,
    Mutating,
    Consuming,
}

/// Marker: can be read as a value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Gettable;

/// Marker: can be written to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Settable;

/// Has body/initializer — CstNode of the body subtree.
#[derive(Clone, Debug)]
pub struct Valued(pub SyntaxNode);

/// Marker: accessed through type, not instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Static;

/// Marker: accessed via call syntax on parent (`obj(key)`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subscript;

// ===== Generics =====

/// Entity IDs of type parameter children.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypeParams(pub Vec<Entity>);

/// Where clause constraints on generic parameters.
#[derive(Clone, Debug)]
pub struct WhereClause(pub Vec<WhereConstraint>);

/// A single constraint in a where clause.
#[derive(Clone, Debug)]
pub enum WhereConstraint {
    /// `T: Protocol` — subject conforms to protocols
    Bound {
        subject: AstType,
        protocols: Vec<AstType>,
        node: SyntaxNode,
    },
    /// `T.Assoc == Concrete` — associated type equality
    Equality {
        lhs: AstType,
        rhs: AstType,
        node: SyntaxNode,
    },
    /// `T: not Protocol` — negative conformance bound
    NegativeBound {
        subject: AstType,
        protocol: AstType,
        node: SyntaxNode,
    },
}

// ===== Type relations =====

/// Conformance list (positive and negative protocol conformances).
#[derive(Clone, Debug)]
pub struct Conformances(pub Vec<ConformanceItem>);

/// A single conformance entry.
#[derive(Clone, Debug)]
pub enum ConformanceItem {
    /// `T: Protocol`
    Positive(AstType, SyntaxNode),
    /// `T: not Protocol`
    Negative(AstType, SyntaxNode),
}

/// The type being extended by an extension declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExtensionTarget(pub AstType);

// ===== Modifiers =====

/// Marker: enum has indirect representation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IsIndirect;

// ===== Metadata =====

/// Attributes on a declaration (e.g. `@inline`, `@available`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attributes(pub Vec<AstAttribute>);

/// A single attribute (e.g. `@inline(always)`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AstAttribute {
    pub name: String,
    pub args: Vec<AstAttributeArg>,
}

/// A single argument within an attribute.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AstAttributeArg {
    pub label: Option<String>,
    pub value: String,
}

/// Documentation comment text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Documentation(pub String);

// ===== Import-specific =====

/// Module path for an import declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModulePath(pub Vec<String>);

/// Alias for a module import (`import Foo as Bar`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImportAlias(pub String);

/// Specific items imported from a module.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImportItems(pub Vec<ImportItem>);

/// A single item from a selective import.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}
