//! Semantic model queries
//!
//! This module contains all query implementations for the SemanticModel.
//! Each query is a struct that implements the Query trait.

mod ancestor_of_kind;
mod child_by_name;
mod declared_names_in_scope;
mod executable_body_for;
mod extension_methods;
mod extensions_for;
mod functions_in_symbol;
mod generics_data_for;
mod has_body;
mod imports_in_scope;
mod inherited_protocol_member;
mod is_inside_any;
mod is_visible_from;
mod resolve_module_path;
mod resolve_name;
mod resolve_type_path;
mod resolve_value_path;
mod resolved_aliased_type;
mod scope_for;
mod struct_fields;
mod struct_methods;
mod symbol_for;
mod visible_children;
mod visible_children_by_name;

pub use ancestor_of_kind::AncestorOfKind;
pub use child_by_name::ChildByName;
pub use declared_names_in_scope::{DeclaredName, DeclaredNamesInScope};
pub use executable_body_for::ExecutableBodyFor;
pub use extension_methods::ExtensionMethods;
pub use extensions_for::ExtensionsFor;
pub use functions_in_symbol::FunctionsInSymbol;
pub use generics_data_for::{GenericsData, GenericsDataFor};
pub use has_body::HasBody;
pub use imports_in_scope::ImportsInScope;
pub use inherited_protocol_member::InheritedProtocolMember;
pub use is_inside_any::IsInsideAny;
pub use is_visible_from::IsVisibleFrom;
pub use resolve_module_path::ResolveModulePath;
pub use resolve_name::ResolveName;
pub use resolve_type_path::ResolveTypePath;
pub use resolve_value_path::ResolveValuePath;
pub use resolved_aliased_type::ResolvedAliasedType;
pub use scope_for::ScopeFor;
pub use struct_fields::{StructFieldInfo, StructFields};
pub use struct_methods::StructMethods;
pub use symbol_for::SymbolFor;
pub use visible_children::VisibleChildren;
pub use visible_children_by_name::VisibleChildrenByName;
