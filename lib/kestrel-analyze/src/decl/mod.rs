//! Declaration-level analyzers -- checks on structural properties.
//!
//! Each analyzer is a stateless ZST that implements `Describe + DeclCheck`.
//! Target kinds control which entity types the analyzer runs on.

pub mod builtin_marker_protocol;
pub mod cloneable_field;
pub mod conformance_rules;
pub mod default_param_ordering;
pub mod duplicate_callable;
pub mod duplicate_case;
pub mod duplicate_deinit;
pub mod duplicate_label;
pub mod duplicate_symbol;
pub mod extension_conflict;
pub mod extension_validation;
pub mod extern_ffi_safe;
pub mod field;
pub mod function_body;
pub mod generics;
pub mod indirect_enum;
pub mod parent_protocol_conformance;
pub mod protocol_field_conformance;
pub mod protocol_method;
pub mod ref_return;
pub mod recursive_enum;
pub mod static_context;
pub mod subscript;
pub mod type_alias_validation;
pub mod visibility;
