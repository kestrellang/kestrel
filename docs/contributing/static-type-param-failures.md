# Static Type Parameter Test Failures

This document tracks the remaining test failures in the static methods on type parameters feature.

## Overview

- **835 tests passing**
- **6 tests failing**
- **4 distinct root causes**

## Failing Tests

### 1. Protocol Init Tests (3 tests)

**Tests:**
- `wrong_argument_labels`
- `wrong_argument_count`
- `ambiguous_init`

**Expected Behavior:**
- `wrong_argument_labels`: `T(wrong: 1)` should fail when protocol expects `init(value: Int)`
- `wrong_argument_count`: `T()` should fail when protocol expects `init(value: Int)`
- `ambiguous_init`: `T()` should fail with "ambiguous" when multiple protocols have `init()`

**Actual Behavior:** All compile successfully without errors.

**Root Cause:** Protocol init declarations (`init(...)` inside a protocol body) are not being parsed. The `ProtocolBodyItem` enum in the parser only supports `Function` and `AssociatedType`, not `Initializer`.

```rust
// lib/kestrel-parser/src/common/data.rs:179-184
pub enum ProtocolBodyItem {
    Function(FunctionDeclarationData),
    AssociatedType(TypeAliasDeclarationData),
    // Missing: Initializer variant
}
```

When `init(...)` is encountered in a protocol body, neither the function parser (expects `func` keyword) nor the type alias parser (expects `type` keyword) matches, so the declaration is silently ignored.

**Required Fix:** Add parser support for protocol initializer declarations:
1. Add `Initializer(InitializerDeclarationData)` variant to `ProtocolBodyItem`
2. Create a protocol-specific initializer parser that allows body-less init declarations
3. Update `protocol_body_item_parser()` to try the initializer parser

---

### 2. Standalone Type Parameter Error (1 test)

**Test:** `standalone_type_parameter_error`

**Expected Behavior:** `let x = T` should produce an error - type parameters cannot be used as standalone values.

**Actual Behavior:** Compiles successfully.

**Root Cause:** When path resolution returns `ValuePathResolution::TypeParameter`, the code creates a `TypeParameterRef` expression without validating the usage context. Type parameters should only be valid when:
- Called as init: `T()`
- Used for member access: `T.method()`

**Location:** `lib/kestrel-semantic-tree-builder/src/body_resolver/paths.rs:160-184`

**Required Fix:** Add context validation when creating `TypeParameterRef`:
```rust
ValuePathResolution::TypeParameter { symbol_id } => {
    // If this is a standalone reference (not being called or accessed),
    // emit TypeParameterCannotBeUsedAsValueError
    if path_with_spans.len() == 1 && !is_being_called {
        emit_error(...);
        return Expression::error(span);
    }
    // ... existing code
}
```

The diagnostic `TypeParameterCannotBeUsedAsValueError` is already defined in `diagnostics/member_access.rs`.

---

### 3. Inherited Protocol Method Lookup (1 test)

**Test:** `static_method_from_inherited_protocol`

**Expected Behavior:** `T.create()` should work when `T: Child` and `Child: Base` where `Base` has `static func create()`.

**Actual Behavior:** Error: "no method 'create' found... none of these protocols have a method named 'create'"

**Root Cause:** `collect_protocol_static_methods` only searches direct children of the protocol, not inherited protocols.

**Location:** `lib/kestrel-semantic-tree-builder/src/body_resolver/members.rs` in `collect_protocol_static_methods`

**Current Code:**
```rust
fn collect_protocol_static_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    ...
) {
    // Only searches protocol.metadata().children()
    // Does NOT traverse inherited protocols
}
```

**Required Fix:** Recursively search inherited protocols:
```rust
fn collect_protocol_static_methods(...) {
    // Search direct children
    for child in protocol.metadata().children() { ... }

    // Also search inherited protocols
    for inherited in protocol.inherited_protocols() {
        collect_protocol_static_methods(inherited, method_name, ...);
    }
}
```

---

### 4. Generic Protocol Bound Validation (1 test)

**Test:** `generic_protocol_bound`

**Expected Behavior:** `T: Container[E]` should produce an error indicating generic protocol bounds are not yet supported.

**Actual Behavior:** Compiles successfully (no error).

**Root Cause:** Generic protocol bounds (protocols with type arguments like `Container[E]`) are parsed but there's no validation rejecting them as unsupported.

**Location:** Type parameter bounds resolution, likely in `lib/kestrel-semantic-tree-builder/src/body_resolver/utils.rs` in `get_type_parameter_bounds_by_id` or related functions.

**Required Fix:** When processing bounds, check if the bound has type arguments and emit `UnsupportedGenericProtocolBoundError` (already defined in `diagnostics/member_access.rs`).

---

## Priority Recommendations

1. **High Priority - Standalone type parameter error**: Simple fix, improves error messages
2. **High Priority - Inherited protocol lookup**: Important for protocol hierarchies
3. **Medium Priority - Generic protocol bound validation**: Good error message improvement
4. **Lower Priority - Protocol init parsing**: Requires significant parser changes

## Related Files

- `lib/kestrel-parser/src/protocol/mod.rs` - Protocol parsing
- `lib/kestrel-parser/src/common/data.rs` - ProtocolBodyItem enum
- `lib/kestrel-semantic-tree-builder/src/body_resolver/paths.rs` - Path resolution
- `lib/kestrel-semantic-tree-builder/src/body_resolver/members.rs` - Member/method resolution
- `lib/kestrel-semantic-tree-builder/src/body_resolver/calls.rs` - Init call resolution
- `lib/kestrel-semantic-tree-builder/src/diagnostics/member_access.rs` - Error definitions
