# Implementation Plan: `lang.ptr[T]` Intrinsic Pointer Type

## Overview

Add `lang.ptr[T]` as a built-in generic pointer type to the Kestrel type system. This is the first step toward supporting low-level memory operations through the `lang` module intrinsics.

## Background

The standard library (`lang/std/memory/pointer.ks`) already uses `lang.ptr[T]` and related intrinsic functions:
- `lang.ptr_null()` - create null pointer
- `lang.ptr_to(value)` - get pointer to a value  
- `lang.ptr_from_address(addr)` - create pointer from integer address
- `lang.ptr_read(ptr)` - read value from pointer
- `lang.ptr_write(ptr, value)` - write value to pointer
- `lang.ptr_offset(ptr, bytes)` - pointer arithmetic

The MIR layer already has `MirTy::Pointer(Id<Ty>)` defined, but there's no corresponding `TyKind::Pointer` in the semantic type system, and `lang.ptr` is not recognized during type resolution.

## Design Approach: Option C (Handle in TypeResolver)

Rather than modifying `ResolveTypePath` or `resolve_primitive_type`, we handle `lang.ptr[T]` directly in `TypeResolver::resolve_ty_path()` before calling the normal resolution path. This keeps the special handling localized and follows the pattern used for other built-in types like `Array`.

Key insight: `lang.ptr[T]` is a **built-in generic type** - similar to `Array` but accessed via a path `lang.ptr` rather than special syntax `[T]`.

## Implementation Steps

### Step 1: Add Tests (TDD)

**Create** `lib/kestrel-test-suite/tests/types/pointer.rs`:
- Basic resolution tests (`lang.ptr[Int]` as type alias, field type, parameter, return type)
- Type argument validation (missing args, too many args, empty brackets)
- Nested and complex types (nested pointers, pointer to tuple/array/struct)
- Generic context (`lang.ptr[T]` where T is a type parameter)
- MIR lowering tests (verify `TyKind::Pointer` lowers to `MirTy::Pointer`)

**Modify** `lib/kestrel-test-suite/tests/types/mod.rs`:
- Add `mod pointer;`

### Step 2: Add `TyKind::Pointer` to Semantic Tree

**File**: `lib/kestrel-semantic-tree/src/ty/kind.rs`

Add after `Array`:
```rust
/// Raw pointer type: lang.ptr[T]
Pointer(Box<Ty>),
```

### Step 3: Add `Ty` Methods for Pointer Type

**File**: `lib/kestrel-semantic-tree/src/ty/mod.rs`

1. Add `Display` case:
```rust
TyKind::Pointer(elem) => write!(f, "lang.ptr[{}]", elem),
```

2. Add constructor:
```rust
/// Create a raw pointer type: lang.ptr[T]
pub fn pointer(element_type: Ty, span: Span) -> Self {
    Self::new(TyKind::Pointer(Box::new(element_type)), span)
}
```

3. Add to `is_type!` macro:
```rust
is_pointer => TyKind::Pointer(_),
```

4. Add accessor:
```rust
/// Get pointer element type if this is a pointer type
pub fn as_pointer(&self) -> Option<&Ty> {
    match &self.kind {
        TyKind::Pointer(element_type) => Some(element_type),
        _ => None,
    }
}
```

5. Update `substitute_self()`:
```rust
TyKind::Pointer(element_type) => {
    let new_element = element_type.substitute_self(replacement);
    Ty::pointer(new_element, self.span.clone())
}
```

6. Update `is_specialization_of()`:
```rust
(TyKind::Pointer(a_elem), TyKind::Pointer(b_elem)) => a_elem.is_specialization_of(b_elem),
```

7. Update `overlaps_with()`:
```rust
(TyKind::Pointer(a_elem), TyKind::Pointer(b_elem)) => a_elem.overlaps_with(b_elem),
```

8. Update `is_assignable_to()`:
```rust
(TyKind::Pointer(a_elem), TyKind::Pointer(b_elem)) => a_elem.is_assignable_to(b_elem),
```

9. Update `is_copyable()` - pointers are copyable:
```rust
TyKind::Pointer(_) => true,
```

10. Update `is_cloneable()` - pointers are not cloneable (they're copyable):
```rust
TyKind::Pointer(_) => false,
```

### Step 4: Add `lang` Module Constants to Prelude

**File**: `lib/kestrel-prelude/src/lib.rs`

Add after `primitives` module:
```rust
/// Lang module intrinsic names
pub mod lang {
    /// The "lang" module name
    pub const LANG: &str = "lang";
    /// Pointer type name
    pub const PTR: &str = "ptr";
}
```

### Step 5: Handle `lang.ptr[T]` in TypeResolver

**File**: `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs`

In `resolve_ty_path()`, before calling `resolve_path()`, add:

```rust
use kestrel_prelude::lang;

// Check for lang.ptr[T] built-in generic
if segments.len() == 2 && segments[0] == lang::LANG && segments[1] == lang::PTR {
    let type_args = self.extract_type_arguments(ty_path_node);
    if type_args.len() != 1 {
        self.diagnostics.throw(LangPtrArityError {
            span: ty_span.clone(),
            got: type_args.len(),
        });
        return Ty::error(ty_span);
    }
    return Ty::pointer(type_args[0].clone(), ty_span);
}
```

**File**: `lib/kestrel-semantic-tree-binder/src/diagnostics/type_resolution.rs`

Add new error type:
```rust
/// Error when lang.ptr has wrong number of type arguments.
pub struct LangPtrArityError {
    pub span: Span,
    pub got: usize,
}

impl IntoDiagnostic for LangPtrArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let message = if self.got == 0 {
            "lang.ptr requires exactly 1 type argument".to_string()
        } else {
            format!("too many type arguments for 'lang.ptr': expected 1, found {}", self.got)
        };
        Diagnostic::error()
            .with_message(message)
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type argument required")
            ])
    }
}
```

**File**: `lib/kestrel-semantic-tree-binder/src/diagnostics/mod.rs`

Export the new error:
```rust
pub use type_resolution::LangPtrArityError;
```

### Step 6: Add Type Lowering to MIR

**File**: `lib/kestrel-execution-graph-lowering/src/ty.rs`

Add case in `lower_type()` after `TyKind::Array`:
```rust
TyKind::Pointer(element_ty) => {
    let element = lower_type(ctx, element_ty);
    ctx.mir.ty_ptr(element)
}
```

### Step 7: Update Monomorphization

**File**: `lib/kestrel-codegen-cranelift/src/monomorphize/substitute.rs`

Add case to substitute type parameters in pointer element types (find where `TyKind::Array` is handled and add similar logic):
```rust
TyKind::Pointer(element) => {
    let new_element = self.substitute_ty(element);
    Ty::pointer(new_element, ty.span().clone())
}
```

**File**: `lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs`

If there's type collection logic that recurses into compound types, ensure `TyKind::Pointer` is handled to recurse into the element type.

## Files Summary

| File | Action | Description |
|------|--------|-------------|
| `lib/kestrel-test-suite/tests/types/pointer.rs` | Create | Test file for lang.ptr[T] |
| `lib/kestrel-test-suite/tests/types/mod.rs` | Modify | Add `mod pointer;` |
| `lib/kestrel-semantic-tree/src/ty/kind.rs` | Modify | Add `TyKind::Pointer(Box<Ty>)` |
| `lib/kestrel-semantic-tree/src/ty/mod.rs` | Modify | Add constructor, accessors, update type methods |
| `lib/kestrel-prelude/src/lib.rs` | Modify | Add `lang` module with `PTR` constant |
| `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs` | Modify | Handle `lang.ptr[T]` specially |
| `lib/kestrel-semantic-tree-binder/src/diagnostics/type_resolution.rs` | Modify | Add `LangPtrArityError` |
| `lib/kestrel-semantic-tree-binder/src/diagnostics/mod.rs` | Modify | Export new error |
| `lib/kestrel-execution-graph-lowering/src/ty.rs` | Modify | Add `TyKind::Pointer` lowering |
| `lib/kestrel-codegen-cranelift/src/monomorphize/substitute.rs` | Modify | Handle pointer substitution |
| `lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs` | Modify | Handle pointer in type collection (if needed) |

## Testing Strategy

1. **Unit tests in test suite** - Comprehensive tests for type resolution and MIR lowering
2. **Run existing tests** - Ensure no regressions with `cargo test`
3. **Manual verification** - Test with sample code using `lang.ptr[T]` as field/parameter/return types

## Future Work

This implementation only adds the `lang.ptr[T]` type. The following are NOT included and will need separate implementations:

1. **Pointer intrinsic functions**: `lang.ptr_null()`, `lang.ptr_to()`, `lang.ptr_read()`, `lang.ptr_write()`, `lang.ptr_offset()`, etc.
2. **Other `lang.*` types**: `lang.u8`, `lang.i32`, etc. (currently handled as primitives `I8`, `I32`, etc.)
3. **Pointer operations in codegen**: The codegen already has `Rvalue::PtrOffset`, `PtrToRef`, etc. but they need to be wired up to the intrinsic functions.

## Error Messages

- `lang.ptr` without type args: "lang.ptr requires exactly 1 type argument"
- `lang.ptr[Int, String]`: "too many type arguments for 'lang.ptr': expected 1, found 2"
- `lang.ptr[]`: "lang.ptr requires exactly 1 type argument" (same as no args since 0 != 1)

## Design Decisions

1. **Copyability**: `lang.ptr[T]` is always copyable (it's just an address). This matches Rust's `*const T` and `*mut T` semantics.

2. **Not cloneable**: Since pointers are copyable, they don't need `.clone()`.

3. **No special parsing**: `lang.ptr[T]` uses standard path + type argument syntax, no parser changes needed.

4. **Localized handling**: The special case is handled in `TypeResolver` only, keeping `ResolveTypePath` generic.
