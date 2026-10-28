//! Hand-written conveniences on the generated views.
//!
//! The generated accessors name each child the grammar declares; this file
//! adds what needs more than that: shared "has a …" traits so one helper can
//! read the same part of every declaration, and text extraction for the
//! small leaf nodes (`Name`, `Path`, `ModulePath`, `Visibility`).

use super::{AstNode, support};
use crate::{SyntaxKind, SyntaxToken};

use super::generated::*;

/// A declaration with a `Name` child.
pub trait HasName: AstNode {
    fn name(&self) -> Option<Name> {
        support::child(self.syntax())
    }

    /// The declared name's text.
    fn name_text(&self) -> Option<String> {
        self.name()?.text()
    }
}

/// A declaration with a `Visibility` child (empty when no keyword is written).
pub trait HasVisibility: AstNode {
    fn visibility(&self) -> Option<Visibility> {
        support::child(self.syntax())
    }
}

/// A declaration that may carry `@attributes`.
pub trait HasAttributes: AstNode {
    fn attribute_list(&self) -> Option<AttributeList> {
        support::child(self.syntax())
    }

    /// Every attribute, in source order.
    fn attributes(&self) -> impl Iterator<Item = Attribute> {
        self.attribute_list()
            .into_iter()
            .flat_map(|list| list.attributes())
    }
}

/// A declaration that may have `[T, …]` and `where …`.
pub trait HasGenerics: AstNode {
    fn type_parameter_list(&self) -> Option<TypeParameterList> {
        support::child(self.syntax())
    }

    fn where_clause(&self) -> Option<WhereClause> {
        support::child(self.syntax())
    }
}

/// A declaration that may have `: P, Q`.
pub trait HasConformances: AstNode {
    fn conformance_list(&self) -> Option<ConformanceList> {
        support::child(self.syntax())
    }
}

/// A declaration that may be `static`.
pub trait HasStatic: AstNode {
    fn is_static(&self) -> bool {
        support::child::<StaticModifier>(self.syntax()).is_some()
    }
}

macro_rules! impl_trait {
    ($tr:ident: $($t:ident),* $(,)?) => { $(impl $tr for $t {})* };
}

impl_trait!(HasName: StructDeclaration, EnumDeclaration, ProtocolDeclaration,
    FunctionDeclaration, FieldDeclaration, TypeAliasDeclaration, EnumCaseDeclaration,
    TypeParameter, EnumCaseParameter, Parameter);
impl_trait!(HasVisibility: StructDeclaration, EnumDeclaration, ProtocolDeclaration,
    FunctionDeclaration, FieldDeclaration, TypeAliasDeclaration, InitializerDeclaration,
    SubscriptDeclaration, EnumCaseDeclaration);
impl_trait!(HasAttributes: StructDeclaration, EnumDeclaration, ProtocolDeclaration,
    FunctionDeclaration, FieldDeclaration, TypeAliasDeclaration, InitializerDeclaration,
    SubscriptDeclaration, EnumCaseDeclaration);
impl_trait!(HasGenerics: StructDeclaration, EnumDeclaration, ProtocolDeclaration,
    FunctionDeclaration, TypeAliasDeclaration, InitializerDeclaration, SubscriptDeclaration,
    ExtensionDeclaration);
impl_trait!(HasConformances: StructDeclaration, EnumDeclaration, ProtocolDeclaration,
    TypeAliasDeclaration, ExtensionDeclaration);
impl_trait!(HasStatic: FunctionDeclaration, FieldDeclaration, SubscriptDeclaration);

impl Name {
    pub fn text(&self) -> Option<String> {
        Some(self.identifier_token()?.text().to_string())
    }
}

impl Path {
    /// The identifier of each element, in order.
    pub fn segments(&self) -> Vec<String> {
        self.segment_tokens()
            .map(|t| t.text().to_string())
            .collect()
    }

    pub fn segment_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        self.path_elements().filter_map(|e| e.identifier_token())
    }
}

impl ModulePath {
    /// The identifier tokens of `A.B.C`, in order.
    pub fn segment_tokens(&self) -> impl Iterator<Item = SyntaxToken> {
        self.syntax()
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::Identifier)
    }
}

/// The four visibility keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityKind {
    Public,
    Private,
    Internal,
    Fileprivate,
}

impl Visibility {
    /// The written keyword, or `None` for an empty `Visibility` node.
    pub fn kind(&self) -> Option<VisibilityKind> {
        let token = super::first_token(self.syntax())?;
        Some(match token.kind() {
            SyntaxKind::Public => VisibilityKind::Public,
            SyntaxKind::Private => VisibilityKind::Private,
            SyntaxKind::Internal => VisibilityKind::Internal,
            SyntaxKind::Fileprivate => VisibilityKind::Fileprivate,
            _ => return None,
        })
    }
}

impl ImportDeclaration {
    /// `import M.(A, B)` — whether an item list was written.
    pub fn has_item_list(&self) -> bool {
        self.l_paren_token().is_some()
    }
}

impl TyFunction {
    /// The kind keyword before the parameter list (`mutating`, `consuming`,
    /// contextual `escaping`), as the token written. Tokens inside the
    /// `TyList` belong to the parameters.
    pub fn kind_token(&self) -> Option<SyntaxToken> {
        self.syntax()
            .children_with_tokens()
            .take_while(|e| e.kind() != SyntaxKind::TyList)
            .filter_map(|e| e.into_token())
            .find(|t| !t.kind().is_trivia())
    }
}

impl TypeAliasDeclaration {
    /// The alias's own name: the last `Name` of a qualified target
    /// (`Item` in `type Iterator.Item = …`), else the plain `Name`.
    pub fn alias_name(&self) -> Option<Name> {
        match self.associated_type_target() {
            Some(target) => target.name(),
            None => HasName::name(self),
        }
    }
}

impl PropertyAccessors {
    pub fn getter(&self) -> Option<GetterClause> {
        support::child(self.syntax())
    }

    pub fn setter(&self) -> Option<SetterClause> {
        support::child(self.syntax())
    }

    pub fn ref_clause(&self) -> Option<RefClause> {
        support::child(self.syntax())
    }

    pub fn mutating_ref_clause(&self) -> Option<MutatingRefClause> {
        support::child(self.syntax())
    }

    /// A getter is declared: a `get { … }` clause or a bodyless `get`
    /// requirement.
    pub fn declares_get(&self) -> bool {
        self.getter().is_some() || self.get_token().is_some()
    }

    /// A setter is declared: a `set { … }` clause or a bodyless `set`
    /// requirement.
    pub fn declares_set(&self) -> bool {
        self.setter().is_some() || self.set_token().is_some()
    }

    /// Any accessor *body* is written. A bodyless block (`{ get }`,
    /// `{ get set }`) only declares how storage may be accessed; a bodied
    /// accessor (`{ get { … } }`, the `{ expr }` shorthand, `{ set { … } }`,
    /// `{ ref { … } }`) replaces storage with computation.
    pub fn has_body(&self) -> bool {
        self.code_block().is_some()
            || self.getter().is_some_and(|g| g.code_block().is_some())
            || self.setter().is_some_and(|s| s.code_block().is_some())
            || self.ref_clause().is_some()
            || self.mutating_ref_clause().is_some()
    }
}
