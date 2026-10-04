//! All component types for declaration entities in the ECS world.
//!
//! Components describe capabilities — what an entity CAN DO. They are
//! orthogonal and composable, derived entirely from the CST during the
//! build (mutation) phase.

use kestrel_hecs::Entity;
use kestrel_span::Span;
use kestrel_syntax_tree::{GreenNode, SyntaxNode, SyntaxNodePtr};

use kestrel_ast::AstType;

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
    /// Setter accessor body for a Field or Subscript. Child of the
    /// declaration it sets. Implicit `newValue` param and Mutating (or
    /// None, for static/global) receiver.
    Setter,
    /// Place-accessor body (`ref { … }` / `mutating ref { … }`) for a
    /// Field or Subscript (stage 1.5). Child of the declaration, like
    /// Setter. Returns the synthesized `&T` / `&mutating T`; the
    /// `MutatingAccessor` marker distinguishes the two kinds (receiver
    /// kind can't — static accessors have no receiver).
    RefAccessor,
    Subscript,
    TypeAlias,
    Import,
    TypeParameter,
    /// Default value expression for a parameter.
    ParamDefault,
}

impl NodeKind {
    /// Whether a declaration of this kind is a **type scope**: its non-static
    /// members get a `self` receiver, and it can host methods, fields and
    /// subscripts.
    ///
    /// Written as an exhaustive `match` on purpose. This predicate had seven
    /// open-coded copies (`matches!(k, Struct | Enum | Protocol | Extension)`)
    /// across the AST builder, HIR lowering, two analyzers and the LSP; adding
    /// a kind meant seven lockstep edits, and missing one reports
    /// "cannot use 'self' in a static method" on correct code. With no
    /// wildcard arm, a new `NodeKind` fails to compile *here* — one place, and
    /// the answer has to be given deliberately.
    pub fn is_type_scope(&self) -> bool {
        match self {
            NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension => true,
            // `EnumCase` is a member of an enum, not a scope of its own.
            NodeKind::Module
            | NodeKind::EnumCase
            | NodeKind::Function
            | NodeKind::Initializer
            | NodeKind::Deinit
            | NodeKind::Field
            | NodeKind::Setter
            | NodeKind::RefAccessor
            | NodeKind::Subscript
            | NodeKind::TypeAlias
            | NodeKind::Import
            | NodeKind::TypeParameter
            | NodeKind::ParamDefault => false,
        }
    }
}

/// Source span excluding leading trivia.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeclSpan(pub Span);

/// Where this declaration is in its file's syntax tree: a `Send`, hashable
/// kind+range handle, never the node itself (a rowan `SyntaxNode` is `!Send`
/// and pins the whole tree — audit F42). Resolve it with
/// [`crate::syntax::cst_node`] (or [`CstNode::to_node`] against the file's
/// [`FileSyntax`] root).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CstNode(pub SyntaxNodePtr);

impl CstNode {
    /// The declaration's node in `root` (its file's tree).
    pub fn to_node(&self, root: &SyntaxNode) -> Option<SyntaxNode> {
        self.0.try_to_node(root)
    }
}

/// A file's parsed syntax tree, on the file entity: the immutable green tree
/// (`Send + Sync`), from which [`FileSyntax::root`] rebuilds a cursor cheaply.
/// Every `CstNode` / `Valued` / conformance / where-clause pointer of the
/// file's declarations resolves against it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileSyntax(pub GreenNode);

impl FileSyntax {
    /// The file's `SourceFile` node.
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.0.clone())
    }
}

// ===== Naming & location =====

/// Declared identifier name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name(pub String);

impl Name {
    /// The name of the world's root entity — the implicit module every
    /// top-level declaration hangs under.
    ///
    /// Deliberately unspellable: `<` and `>` are not identifier characters, so
    /// no source module can collide with it.
    ///
    /// **Compare with [`Name::is_root`], never with a literal.** The check
    /// fails *open* — `visibility.rs` decides whether a declaration is
    /// top-level by asking whether its parent is root, and a miss makes every
    /// `internal` declaration universally visible with no diagnostic. A
    /// literal at a call site is a copy that a rename cannot reach.
    pub const ROOT: &'static str = "<root>";

    /// Whether this is the root entity's name. See [`Name::ROOT`].
    pub fn is_root(&self) -> bool {
        self.0 == Self::ROOT
    }
}

/// Source file entity this declaration belongs to.
/// Modules don't get FileId — they span multiple files.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileId(pub Entity);

/// Display path for the file entity (set by the compiler when a source is added).
/// Used for diagnostics and for resolving `@fileconstant` paths relative to the
/// source file's directory.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FilePath(pub String);

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

/// Marks an init as failable (`?`) or throwing (`throws E`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InitEffect {
    Failable,
    Throwing,
}

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
    /// Entity for the default value expression (child entity with `Valued` + TypeAnnotation).
    pub default_entity: Option<Entity>,
    /// Destructuring pattern for this parameter, if any.
    /// None for simple binding parameters (`x: Int`).
    /// Some for destructured parameters (`(a, b): (Int, Int)`).
    pub pattern: Option<ParamPattern>,
    /// Whether this parameter has mutating or consuming access mode.
    pub is_mut: bool,
    /// Whether this parameter specifically has consuming access mode.
    pub is_consuming: bool,
}

/// A parameter destructuring pattern — lightweight tree that doesn't need arena allocation.
/// Used to bridge build phase (AstParam) to query phase (HIR lowering).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ParamPattern {
    Binding {
        name: String,
        is_mut: bool,
    },
    Tuple {
        elements: Vec<ParamPattern>,
    },
    Struct {
        type_name: String,
        fields: Vec<StructPatternField>,
        has_rest: bool,
    },
    Wildcard,
}

/// A field in a struct destructuring pattern.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StructPatternField {
    pub field_name: String,
    pub pattern: ParamPattern,
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

/// Marker on a `NodeKind::RefAccessor` entity: this is the `mutating ref`
/// accessor (returns `&mutating T`, Mutating receiver) rather than the
/// shared `ref` accessor (returns `&T`, Borrowing receiver).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MutatingAccessor;

/// Has a body to lower — a handle to the body subtree: a `CodeBlock`, a
/// function's `= expr` (`FunctionBody`), a parameter default's `= expr`
/// (`DefaultValue`), or a field initializer `Expression`. Resolve it with
/// [`crate::syntax::valued_node`]; `kestrel-hir-lower`'s `LowerBody` lowers
/// it on demand. This is the one "has a body" marker — every body entity
/// carries it, and nothing else does.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Valued(pub SyntaxNodePtr);

impl Valued {
    /// The body's node in `root` (its file's tree).
    pub fn to_node(&self, root: &SyntaxNode) -> Option<SyntaxNode> {
        self.0.try_to_node(root)
    }
}

/// Marker: accessed through type, not instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Static;

/// Marker: accessed via call syntax on parent (`obj(key)`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subscript;

/// The enclosing type-level container (Struct, Enum, Extension, Protocol,
/// Module) for entities that sit more than one hop below it in the tree.
/// Set at build time on Setter entities (Setter → Subscript/Field → Container)
/// so downstream code doesn't need to walk parents and remember the extra hop.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EnclosingContainer(pub Entity);

/// Whether a Field was declared with `var` (mutable) or `let` (read-only).
/// Captured at build time so downstream analyzers don't have to inspect
/// tokens. Only present on entities with NodeKind::Field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldMutability {
    Var,
    Let,
}

/// Marker: this Field declares an accessor block (`{ get }`, `{ get set }`,
/// `{ get { … } }`, `{ ref { … } }`, …), bodyless or not. Set by the field
/// builder when the CST has PropertyAccessors.
///
/// This is the *accessor-shape* question (drives E413, E622, doc rendering).
/// It is NOT the storage question — a bodyless block on a concrete type
/// declares access to storage, not a replacement for it. For storage, read
/// [`FieldClass`]; never infer it from the absence of this marker.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Computed;

/// How a [`NodeKind::Field`] is backed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldBacking {
    /// Occupies storage — inline in the instance, or a global for `static` and
    /// module-level fields. A *bodyless* accessor block (`var b: Int { get set }`)
    /// on a concrete type is Stored: the block declares access, not a body.
    Stored,
    /// Backed by accessor bodies — a bodied `get`/`set`, the `{ expr }`
    /// shorthand, or a `ref`/`mutating ref` clause. Occupies no storage.
    Computed,
}

/// Which kind of container a [`NodeKind::Field`] is declared in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldOwner {
    /// A `struct` or `enum` — the only owner whose stored fields get a FieldIdx.
    Nominal,
    /// A `protocol`: a requirement to witness, never storage, whatever its backing.
    Protocol,
    /// An `extend` block.
    Extension,
    /// Module level. A module-level `var g = 0;` is a global *without* carrying
    /// [`Static`], which is why global-ness is `is_static || owner == Module`
    /// rather than a `Static` test.
    Module,
}

/// The storage classification of a [`NodeKind::Field`], computed once by the
/// field builder — the only code that inspects the accessor CST — and stored
/// rather than re-derived.
///
/// Downstream code must read this instead of reconstructing storage-ness from
/// the *absence* of [`Computed`]/[`Callable`]/[`Static`]. Classification by
/// absent marker is precisely what caused the F3 divergence: [`Computed`] is
/// set for any accessor block but [`Callable`] only for a bodied one, so
/// `!Callable` and `!Computed` disagreed and the layout, memberwise-init and
/// pattern-arity rosters drifted apart — silently, because struct construction
/// maps argument position to field index with no name check.
///
/// Use the helpers; do not match the fields ad hoc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldClass {
    pub backing: FieldBacking,
    pub owner: FieldOwner,
    /// The `static` modifier only. Orthogonal to `owner`.
    pub is_static: bool,
}

impl FieldClass {
    /// Occupies inline storage in an instance, i.e. gets a `FieldIdx` in the
    /// type's layout. THE storage test — layout, memberwise init, pattern
    /// arity, and the copy/drop folds must all agree with this and nothing else.
    pub fn is_stored_instance(&self) -> bool {
        self.backing == FieldBacking::Stored
            && !self.is_static
            && matches!(self.owner, FieldOwner::Nominal)
    }

    /// Backed by a `GlobalRef` rather than by instance storage.
    pub fn is_global_storage(&self) -> bool {
        self.backing == FieldBacking::Stored
            && (self.is_static || matches!(self.owner, FieldOwner::Module))
    }

    /// A protocol requirement: witness-dispatched, storage nowhere.
    pub fn is_protocol_requirement(&self) -> bool {
        matches!(self.owner, FieldOwner::Protocol)
    }
}

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
        node: SyntaxNodePtr,
    },
    /// `T.Assoc == Concrete` — associated type equality
    Equality {
        lhs: AstType,
        rhs: AstType,
        node: SyntaxNodePtr,
    },
    /// `T: not Protocol` — negative conformance bound
    NegativeBound {
        subject: AstType,
        protocol: AstType,
        node: SyntaxNodePtr,
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
    Positive(AstType, SyntaxNodePtr),
    /// `T: not Protocol`
    Negative(AstType, SyntaxNodePtr),
}

/// The type being extended by an extension declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExtensionTarget(pub AstType);

/// Names the extension's target LHS *introduces* as type parameters —
/// `extend Pair[T, U]` → `["T", "U"]`, `extend Pair[Int64, U]` → `["U"]`,
/// `extend Int64` → `[]`. For a ref target `extend &T` the pointee is the
/// parameter position, so → `["T"]`.
///
/// The entities these names bind belong to the *target nominal*, not to the
/// extension, so an extension carries no `TypeParams` for them. This component
/// is the single record of which of the target's parameters the LHS actually
/// bound; without it, consumers either miss them entirely (E439's shadowing
/// walk) or over-approximate by taking every parameter the target declares
/// (which leaks `T` into `extend Box[Int64]` bodies).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExtensionLhsParamNames(pub Vec<String>);

/// On a TypeAlias declared as `type Protocol.Assoc = T` — the qualifying
/// protocol path. Absence means the alias is unqualified (`type Assoc = T`).
/// Captured at build time so analyzers can resolve the protocol entity via
/// ResolveTypePath instead of walking CST tokens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct QualifiedTarget(pub AstType);

// ===== Modifiers =====

/// Marker: enum has indirect representation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IsIndirect;

/// Marker: this is a compiler-intrinsic entity (lang module type or function).
/// Codegen handles these specially — they don't have real bodies or layouts.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Intrinsic;

// ===== Metadata =====

/// Attributes on a declaration (e.g. `@inline`, `@available`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Attributes(pub Vec<AstAttribute>);

/// A single attribute (e.g. `@inline(always)`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AstAttribute {
    pub name: String,
    pub args: Vec<AstAttributeArg>,
    /// Span of the full attribute (including `@` and args).
    pub span: Span,
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

/// Marker: this ParamDefault entity's expression references a sibling parameter.
/// Stores the referenced parameter name for diagnostic messages.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DefaultReferencesParam(pub String);

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

// ===== Compilation target =====

/// Compilation target for conditional filtering via `@platform`, etc.
/// Extensible: new dimensions (arch, features) can be added as fields.
#[derive(Clone, Debug)]
pub struct TargetConfig {
    pub os: Option<Os>,
}

impl TargetConfig {
    /// Detect the target from the current host platform.
    pub fn host() -> Self {
        Self {
            os: Some(Os::host()),
        }
    }
}

/// Target operating system for `@platform(.darwin)` / `@platform(.linux)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Os {
    Darwin,
    Linux,
}

impl Os {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "darwin" => Some(Os::Darwin),
            "linux" => Some(Os::Linux),
            _ => None,
        }
    }

    pub fn host() -> Self {
        match std::env::consts::OS {
            "macos" => Os::Darwin,
            "linux" => Os::Linux,
            other => panic!("unsupported platform: {}", other),
        }
    }
}
