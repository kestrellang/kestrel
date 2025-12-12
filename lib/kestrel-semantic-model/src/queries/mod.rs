//! Semantic model queries
//!
//! This module contains all query implementations for the SemanticModel.
//! Each query is a struct that implements the Query trait.

mod ancestor_of_kind;
mod child_by_name;
mod extensions_for;
mod imports_in_scope;
mod inherited_protocol_member;
mod is_visible_from;
mod resolve_module_path;
mod resolve_name;
mod resolve_type_path;
mod resolve_value_path;
mod scope_for;
mod symbol_for;
mod visible_children;
mod visible_children_by_name;

pub use ancestor_of_kind::AncestorOfKind;
pub use child_by_name::ChildByName;
pub use extensions_for::ExtensionsFor;
pub use imports_in_scope::ImportsInScope;
pub use inherited_protocol_member::InheritedProtocolMember;
pub use is_visible_from::IsVisibleFrom;
pub use resolve_module_path::ResolveModulePath;
pub use resolve_name::ResolveName;
pub use resolve_type_path::ResolveTypePath;
pub use resolve_value_path::ResolveValuePath;
pub use scope_for::ScopeFor;
pub use symbol_for::SymbolFor;
pub use visible_children::VisibleChildren;
pub use visible_children_by_name::VisibleChildrenByName;
