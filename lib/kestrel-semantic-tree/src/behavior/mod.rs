pub mod attributes;
pub mod callable;
pub mod computed_member_access;
pub mod conformances;
pub mod conforms_to;
pub mod copy_semantics;
pub mod deinit;
pub mod executable;
pub mod extension_target;
pub mod extern_fn;
pub mod file_constant;
pub mod function_data;
pub mod generics;
pub mod implements;
pub mod member_access;
pub mod subscript;
pub mod typed;
pub mod valued;
pub mod visibility;

pub use computed_member_access::ComputedMemberAccessBehavior;
pub use file_constant::FileConstantBehavior;
pub use subscript::SubscriptBehavior;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KestrelBehaviorKind {
    AssociatedTypeBounds,
    Attributes,
    Callable,
    ComputedMemberAccess,
    Conformances,
    ConformsTo,
    CopySemantics,
    Deinit,
    Executable,
    Extern,
    ExtensionTarget,
    FileConstant,
    FlattenedProtocol,
    FunctionData,
    Generics,
    Implements,
    ImportData,
    MemberAccess,
    ResolvedExecutable,
    Subscript,
    Typed,
    TypeAliasTyped,
    Valued,
    Visibility,
}
