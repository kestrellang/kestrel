//! Declaration-level analyzers -- checks on structural properties.
//!
//! Each analyzer is a stateless ZST that implements `Describe + DeclCheck`.
//! Target kinds control which entity types the analyzer runs on.

pub mod attribute_semantics;
pub mod builtin_marker_protocol;
pub mod cloneable_field;
pub mod closure_field;
pub mod closure_kind_convention;
pub mod conformance_rules;
pub mod conformance_target;
pub mod default_param_ordering;
pub mod derive_site;
pub mod duplicate_builtin;
pub mod duplicate_callable;
pub mod duplicate_case;
pub mod duplicate_deinit;
pub mod duplicate_label;
pub mod duplicate_symbol;
pub mod extension_conflict;
pub mod extension_validation;
pub mod extern_ffi_safe;
pub mod field;
pub mod field_method_collision;
pub mod function_body;
pub mod generics;
pub mod import_target;
pub mod indirect_enum;
pub mod parent_protocol_conformance;
pub mod place_accessor;
pub mod protocol_field_conformance;
pub mod protocol_method;
pub mod recursive_enum;
pub mod ref_return;
pub mod restricted_signature;
pub mod static_context;
pub mod static_value_type;
pub mod subscript;
pub mod type_alias_validation;
pub mod unsized_param;
pub mod existential_box;
pub mod visibility;
