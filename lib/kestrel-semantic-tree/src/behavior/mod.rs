pub mod attributes;
pub mod callable;
pub mod computed_marker;
pub mod computed_member_access;
pub mod conformances;
pub mod conforms_to;
pub mod copy_semantics;
pub mod deinit;
pub mod doc_comment;
pub mod executable;
pub mod extension_target;
pub mod extern_fn;
pub mod file_constant;
pub mod function_data;
pub mod generics;
pub mod implements;
pub mod markers;
pub mod member_access;
pub mod static_marker;
pub mod subscript;
pub mod typed;
pub mod valued;
pub mod visibility;

pub use computed_marker::ComputedPropertyMarker;
pub use computed_member_access::ComputedMemberAccessBehavior;
pub use file_constant::FileConstantBehavior;
pub use markers::{
    AccessorMarker, AccessorParentMarker, CallableScopeMarker, ConcreteTypeMarker,
    HasMembersMarker, MethodContainerMarker, NamespaceScopeMarker,
};
pub use static_marker::StaticBehavior;
pub use subscript::SubscriptBehavior;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KestrelBehaviorKind {
    Accessor,
    AccessorParent,
    AssociatedTypeBounds,
    Attributes,
    Callable,
    CallableScope,
    ComputedMemberAccess,
    ComputedProperty,
    ConcreteType,
    Conformances,
    ConformsTo,
    CopySemantics,
    Deinit,
    DocComment,
    Executable,
    Extern,
    ExtensionTarget,
    FileConstant,
    FlattenedProtocol,
    FunctionData,
    Generics,
    HasMembers,
    Implements,
    ImportData,
    MemberAccess,
    MethodContainer,
    NamespaceScope,
    QualifiedBinding,
    ResolvedExecutable,
    Static,
    Subscript,
    Typed,
    TypeAliasTyped,
    Valued,
    Visibility,
}
