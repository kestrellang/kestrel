//! kestrel-name-res: Name resolution for the ECS-based compiler pipeline.
//!
//! Resolves textual names to ECS entities. Sits between AST building
//! (which creates declaration entities) and HIR lowering (which needs
//! entity references for paths and types).
//!
//! All resolution is implemented as incremental queries against the
//! kestrel-hecs world.

pub mod conformances;
pub mod extensions;
pub mod helpers;
pub mod protocol_members;
pub mod type_members;
pub mod resolve_builtin;
pub mod resolve_module;
pub mod resolve_name;
pub mod resolve_type;
pub mod resolve_value;
pub mod scope;
pub mod visibility;

// Re-export primary query types
pub use conformances::{
    ConformingProtocolInstantiations, ConformingProtocols, expand_protocol_closure,
    expand_protocol_closure_in_place, extract_ast_type_args, find_protocol_witness_init,
};
pub use extensions::{ExtensionTargetEntity, ExtensionsFor, ResolvedExtensionTarget};
pub use protocol_members::{
    ProtocolAssociatedTypes, ProtocolMember, ProtocolMembers, ProtocolMembersByName,
};
pub use type_members::{TypeMember, TypeMemberSource, TypeMembers, TypeMembersByName};
pub use resolve_builtin::{BuiltinIndex, BuiltinMap, EntityBuiltin, ResolveBuiltin};
pub use resolve_module::{ResolveModulePath, StdModules};
pub use resolve_name::{NameResolution, ResolveName};
pub use resolve_type::{ResolveTypePath, TypeResolution};
pub use resolve_value::{ResolveValuePath, ValueResolution};
pub use scope::{Scope, ScopeFor};
pub use visibility::{IsVisibleFrom, VisibleChildrenByName};
