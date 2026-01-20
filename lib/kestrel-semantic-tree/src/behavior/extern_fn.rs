//! Extern function behavior for FFI declarations.
//!
//! This behavior is attached to functions declared with `@extern(.C)` to indicate
//! they are external C functions that will be linked at compile time.

use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;

/// Calling conventions for extern functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    /// C calling convention (cdecl on most platforms)
    C,
}

impl CallingConvention {
    /// Get the name of this calling convention for display.
    pub fn name(&self) -> &'static str {
        match self {
            CallingConvention::C => "C",
        }
    }
}

/// Behavior for extern functions declared with `@extern(.C)`.
///
/// This behavior indicates that the function:
/// - Has no body (implementation is external)
/// - Uses the specified calling convention
/// - Will be linked from external code at compile time
///
/// # Example
///
/// ```kestrel
/// @extern(.C)
/// func malloc(size: UInt) -> RawPointer {}
///
/// @extern(.C, mangleName: "my_c_function")
/// func myFunction(x: Int32) -> Int32 {}
/// ```
#[derive(Debug, Clone)]
pub struct ExternBehavior {
    /// The calling convention for this extern function
    calling_convention: CallingConvention,
    /// Optional custom symbol name for linking (mangleName parameter)
    mangle_name: Option<String>,
}

impl ExternBehavior {
    /// Create a new ExternBehavior.
    pub fn new(calling_convention: CallingConvention, mangle_name: Option<String>) -> Self {
        Self {
            calling_convention,
            mangle_name,
        }
    }

    /// Get the calling convention.
    pub fn calling_convention(&self) -> CallingConvention {
        self.calling_convention
    }

    /// Get the custom mangle name, if specified.
    pub fn mangle_name(&self) -> Option<&str> {
        self.mangle_name.as_deref()
    }

    /// Get the symbol name to use for linking.
    ///
    /// Returns the mangle_name if specified, otherwise falls back to the
    /// provided function name.
    pub fn symbol_name<'a>(&'a self, function_name: &'a str) -> &'a str {
        self.mangle_name.as_deref().unwrap_or(function_name)
    }
}

impl Behavior<KestrelLanguage> for ExternBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Extern
    }
}
