# Known Compiler Issues

This document tracks known bugs and limitations in the Kestrel compiler.

---

## Parser Issues

### ~~Trailing comma before brace causes parse error~~ (Fixed)

**Error:** `found 'LBrace' expected something else`

**Cause:** A trailing comma after the last protocol conformance before the opening brace:
```kestrel
public struct Int8:
    Convertible[Int16],
    Convertible[Int32],  // <- trailing comma here
{
```

**Status:** Fixed - trailing commas are now allowed in conformance lists.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::trailing_comma_in_conformance_list`

---

### ~~Computed properties in enums parsed as free functions~~ (Fixed)

**Error:** `cannot use 'self' in free function`

**Cause:** Computed properties in enums weren't getting `self` injected into their local scope during semantic analysis (the getter binder only handled structs and extensions, not enums).

**Status:** Fixed - enum computed properties now work correctly.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/enums.rs::regression::enum_computed_property_can_use_self`

---

## Type Alias Issues

### ~~Type aliases don't work for member access~~ (Fixed)

**Error:** `cannot find type 'Int' in this scope`

**Cause:** Type aliases like `public type Int = Int64` could not be used to access members or call constructors through the alias name.

**Status:** Fixed - type aliases now correctly resolve through to their underlying type for static member access and initializer calls.

**Regression tests:**
- `lib/kestrel-test-suite/tests/declarations/type_aliases.rs::regression::type_alias_static_method_access`
- `lib/kestrel-test-suite/tests/declarations/type_aliases.rs::regression::type_alias_init_call`

---

### ~~Type aliases don't expose methods~~ (Fixed)

**Error:** `cannot access member on type 'GlobalAllocator'`

**Cause:** Type aliases didn't allow method calls through the alias:
```kestrel
public type GlobalAllocator = SystemAllocator

var allocator: GlobalAllocator = GlobalAllocator();
allocator.allocate(layout)  // Error: member access not supported
```

**Status:** Fixed - type aliases now correctly expand to their underlying type for instance method calls.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/type_aliases.rs::regression::type_alias_instance_method_call`

---

## Type Inference Issues

### ~~Type inference fails with untyped lang intrinsics~~ (Fixed)

**Error:** `could not infer type for 1 placeholder(s)`

**Cause:** `lang.ptr_null()` returns an untyped pointer (`Pointer[Ty::infer()]`), and wrapping it in `lang.cast_ptr[T]()` didn't constrain the inference variable because both the argument and parameter had unconstrained inference variables that were unified together.

**Status:** Fixed - `lang.cast_ptr[T](lang.ptr_null())` now works correctly.

**Regression tests:**
- `lib/kestrel-test-suite/tests/types/pointer.rs::regression::cast_ptr_with_untyped_ptr_null`
- `lib/kestrel-test-suite/tests/types/pointer.rs::regression::cast_ptr_in_generic_context`
- `lib/kestrel-test-suite/tests/types/pointer.rs::regression::cast_ptr_with_various_primitives`

**Root cause and fix:**

When `lang.cast_ptr[T](lang.ptr_null())` was called:
1. `lang.ptr_null()` created a `LangIntrinsic::PtrNull { pointee_ty: Ty::infer() }` with return type `Pointer[Ty::infer()]`
2. This was passed to `lang.cast_ptr[T]` which has signature `(Pointer[_]) -> Pointer[T]` (where `_` is also an inference variable)
3. Type inference unified `Pointer[Ty::infer()]` with `Pointer[_]`, but since both were unconstrained inference variables, neither was resolved
4. After type inference, the unresolved placeholder remained, causing the error

The fix was implemented in two parts:

1. **Constraint generation** (`lib/kestrel-semantic-type-inference/src/constraint_generator.rs`): Added logic to the `LangIntrinsic` constraint generator to equate the argument's pointee type with the target type when `cast_ptr[T]` has a concrete target type T. This creates the constraint: `arg_pointee.id() == target_ty.id()`.

2. **Type application** (`lib/kestrel-semantic-type-inference/src/apply.rs`): Added a `resolve_intrinsic` function that extracts the pointee type from the expression's resolved type when applying solutions. This ensures that pointer-returning intrinsics (`PtrNull`, `PtrFromAddress`, `PtrTo`, `CastPtr`) have their embedded type parameters properly resolved.

The key insight is that `cast_ptr[T]` should propagate its type parameter to constrain the source pointer's pointee type, even though semantically it doesn't care about the source type. This enables patterns like `cast_ptr[T](ptr_null())` to work by inferring that `ptr_null()` should return `Pointer[T]`.

---

### ~~Generic init with where clause not supported~~ (Fixed)

**Error:** `cannot find type 'I' in this scope`

**Cause:** Generic initializers with where clauses in protocols weren't getting their type parameters registered as child symbols during the BUILD phase of semantic analysis. The `InitializerBuilder` was missing the type parameter extraction that `FunctionBuilder` and `SubscriptBuilder` had.

**Status:** Fixed - generic initializers now properly extract and register type parameters.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/protocols.rs::regression::generic_init_with_where_clause`

---

### ~~Static generic methods cause type parameter confusion~~ (Fixed)

**Error:** `type parameter 'T' shadows one from outer scope`

**Cause:** When a static method declared a type parameter with the same name as the struct's type parameter (e.g., `struct Pointer[T] { static func nilPointer[T]() }`), the compiler rejected it as shadowing, even though static methods don't have access to the struct's type parameters.

**Status:** Fixed - static methods can now use the same type parameter names as their containing type.

**Root cause and fix:**

The type parameter shadowing check in `check_duplicate_type_parameters` didn't distinguish between static and instance methods. The fix modifies the shadowing check to allow static methods to shadow type parameters from their immediate parent (struct/enum), since those type parameters are not accessible in the static method's scope anyway.

The implementation:
1. Added `is_static_method()` helper to check if a symbol is a static function
2. Added `is_from_immediate_parent()` to check if a type parameter belongs to the method's parent
3. Modified the shadowing check to skip the error when:
   - The method is static
   - The shadowed type parameter is from the immediate parent (struct/enum)

This allows valid patterns like:
```kestrel
struct Pointer[T] {
    static func nilPointer[T]() -> Pointer[T] {
        return Pointer(raw: lang.ptr_null[T]())
    }
}

func example[T]() -> Pointer[T] {
    return Pointer.nilPointer[T]()  // Now works!
}
```

Instance methods still correctly reject shadowing, since they have access to the struct's type parameters.

**Regression tests:**
- `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::static_method_can_shadow_struct_type_parameter`
- `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::instance_method_cannot_shadow_struct_type_parameter`

---

### ~~`lang.panic` return type `!` doesn't unify with other branch types~~ (Fixed)

**Error:** `type '!' does not conform to protocol 'ExpressibleByBoolLiteral'`

**Cause:** When an if-else expression has a concrete type in one branch and `lang.panic` (which returns `!` never type) in the other, the compiler couldn't unify the types:
```kestrel
public mutating func insert(element: T) -> Bool {
    if maybeSlot.isSome() {
        true  // Bool
    } else {
        lang.panic("...")  // Returns `!`
    }
}
```

**Status:** Fixed - the Never type (`!`) now correctly unifies with any other type during type inference, as it is a bottom type.

**Regression test:** `lib/kestrel-test-suite/tests/expressions/control_flow.rs::regression::never_type_unifies_with_concrete_types_in_if_else`

**Root cause and fix:**

The Never type (`!`) is a "bottom type" in Kestrel's type system - it represents computations that never return normally (like `panic`, `return`, or infinite loops). As a bottom type, it should be compatible with any other type.

The fix was implemented in the type inference unification algorithm in `lib/kestrel-semantic-type-inference/src/solver.rs`. The `unify` function now includes a special case for the Never type:

```rust
// Never unifies with anything (bottom type)
(TyKind::Never, _) | (_, TyKind::Never) => Ok(SolveResult::Solved),
```

This allows the type inference system to correctly handle if-else expressions where one branch has a concrete type and the other has Never. When unifying:
- `Bool` with `Never` → result type is `Bool`
- `Never` with `i64` → result type is `i64`
- `Never` with any type `T` → result type is `T`

The fix was part of the initial type inference system implementation (commit 55e49d6e, December 2025).

---

### ~~Integer literal type inference in match~~ (Fixed)

**Error:** `type mismatch: expected 'I64', found 'Int32'`

**Cause:** Integer literal patterns in match expressions always defaulted to I64, regardless of the scrutinee type. This caused type mismatches when matching against other integer types.

**Status:** Fixed. Integer literal patterns now use type inference to match against:
- Primitive integer types (lang.i8, lang.i16, lang.i32, lang.i64, etc.)
- Wrapper struct types that conform to `ExpressibleByIntLiteral`

**Regression tests:**
- `lib/kestrel-test-suite/tests/patterns/match_expressions.rs::regression::integer_literal_pattern_inherits_primitive_type`
- `lib/kestrel-test-suite/tests/patterns/match_expressions.rs::regression::integer_literal_pattern_with_primitive_i8`
- `lib/kestrel-test-suite/tests/patterns/match_expressions.rs::regression::integer_literal_pattern_with_primitive_i16`
- `lib/kestrel-test-suite/tests/patterns/match_expressions.rs::regression::integer_literal_pattern_with_wrapper_struct`

**Root cause and fix:**

The original fix only handled primitive types by checking `expected_ty` during pattern binding. The complete fix uses the type inference system:

1. **`lib/kestrel-semantic-tree-binder/src/body_resolver/patterns.rs`**: Changed `resolve_literal_pattern` to create an inference placeholder (`Ty::infer()`) for integer literal patterns instead of hardcoding to I64.

2. **`lib/kestrel-semantic-type-inference/src/constraint_generator.rs`**: Added `ExpressibleByIntLiteral` protocol constraint for literal patterns in `generate_pattern_constraints`. This mirrors how literal expressions work.

3. The type inference solver then:
   - Unifies the pattern's inferred type with the scrutinee type
   - Checks that the scrutinee type conforms to `ExpressibleByIntLiteral`
   - If conformance exists, the match compiles successfully

---

## Protocol Issues

### ~~Child protocol cannot redeclare parent's associated type~~ (Fixed)

**Error:** `conflicting associated type 'Element' from inherited protocols`

**Cause:** When a protocol inherits from another protocol and redeclares an associated type with the same name, the compiler incorrectly treated this as a conflict error. The protocol flattener in `lib/kestrel-semantic-tree-binder/src/binders/protocol_flattener.rs` (lines 144-157) didn't distinguish between:
1. A child protocol refining/redeclaring a parent's associated type (should be allowed)
2. Two sibling protocols defining the same associated type (diamond inheritance conflict - should error)

**Status:** Fixed - child protocols can now redeclare parent associated types, while diamond inheritance conflicts are still properly detected and reported.

**Root cause and fix:**

The issue was in the `flatten_protocol_recursive` function. When processing a protocol's associated types, if an associated type with the same name already existed in the flattened map, the code would immediately throw an error. This happened because parent protocols are processed before child protocols (recursive flattening), so by the time a child's associated type is encountered, the parent's version is already in the map.

The fix adds an `is_ancestor_protocol` helper function that checks if the existing associated type came from an ancestor protocol by recursively walking up the protocol hierarchy. When a duplicate associated type is found:
- If it's from an ancestor → allow the child's declaration to override it
- If it's from a sibling protocol → throw the diamond inheritance conflict error

This preserves the legitimate error case (diamond inheritance) while allowing the common pattern of child protocols refining parent associated types.

**Regression tests:**
- `lib/kestrel-test-suite/tests/declarations/protocols.rs::regression::child_protocol_can_redeclare_parent_associated_type`
- `lib/kestrel-test-suite/tests/declarations/protocols.rs::regression::diamond_inheritance_associated_type_conflict`

---

### ~~Protocol extension default implementations not inherited~~ (Fixed)

**Error:** `'Array' conforms to 'ExpressibleByArrayLiteral' but not its parent protocol '_ExpressibleByArrayLiteral'`

**Cause:** When a protocol extends another protocol and provides a default implementation via `extend`, types conforming to the child protocol didn't automatically get the default implementation. This was due to multiple phases failing to consider protocol extensions:
1. `validate_parent_protocol_conformances` required explicit conformance to parent protocols without checking if default implementations exist
2. `check_struct_conformance` only collected methods from struct extensions, not from protocol extensions
3. `ProtocolRequiredMethods` query didn't exclude methods with default implementations from protocol extensions

**Status:** Fixed - protocol extension default implementations are now properly inherited.

**Fix:** Modified `ProtocolRequiredMethods` query in `lib/kestrel-semantic-model/src/queries/protocol_required_methods.rs` to:
1. Collect methods from protocol extensions using the `ExtensionsFor` query
2. Exclude those methods from the "required" set since they have default implementations
3. Updated `validate_parent_protocol_conformances` in `lib/kestrel-semantic-tree-binder/src/syntax/helpers.rs` to skip parent conformance validation when all parent protocol methods have default implementations

**Regression test:** `lib/kestrel-test-suite/tests/declarations/protocols.rs::regression::protocol_extension_default_implementation`

---

## Name Resolution Issues

### ~~Name conflict between protocol and enum case~~ (Fixed)

**Error:** `'Equal' is not a type` / `undefined name 'Equal'`

**Cause:** Enum cases were hardcoded to have Internal visibility in `enum_case.rs`, regardless of their parent enum's visibility. When a public enum was imported in another module, its cases were invisible because they had Internal visibility, causing name resolution errors.

**Status:** Fixed - enum cases now inherit their parent enum's visibility.

**Root cause and fix:**

The `EnumCaseBuilder` in `lib/kestrel-semantic-tree-builder/src/builders/enum_case.rs` was hardcoding visibility to `Internal` at line 41:

```rust
// Before (buggy):
let visibility_behavior = VisibilityBehavior::new(
    Some(Visibility::Internal),  // Always Internal!
    name_span.clone(),
    visibility_scope,
);
```

The fix retrieves the parent enum's visibility and uses it for the case:

```rust
// After (fixed):
let parent_visibility = parent
    .and_then(|p| p.metadata().get_behavior::<VisibilityBehavior>())
    .and_then(|v| v.visibility().copied());

let visibility_scope = find_visibility_scope(parent_visibility.as_ref(), parent, root);
let visibility_behavior = VisibilityBehavior::new(
    parent_visibility,
    name_span.clone(),
    visibility_scope,
);
```

**Regression tests:**
- `lib/kestrel-test-suite/tests/declarations/enums.rs::regression::enum_cases_inherit_parent_visibility`
- `lib/kestrel-test-suite/tests/declarations/enums.rs::regression::public_enum_cases_accessible_across_modules`

---

### ~~Cross-module enum shorthand resolution fails~~ (Fixed)

**Error:** `no matching overload for 'Ok'` (provided `(_)`, expected `(value)`)

**Cause:** Using `.Ok(value)` or `.Err(error)` shorthand syntax failed because the type inference constraint solver required argument labels to match exactly. When calling `.Ok(buf)` with an unlabeled argument, the solver expected a `value:` label because the enum case was defined as `case Ok(value: T)`.

**Status:** Fixed - enum shorthand syntax now allows unlabeled arguments to match labeled parameters using positional matching, which is consistent with how function calls work in Kestrel.

**Root cause and fix:** The `resolve_implicit_member` function in `lib/kestrel-semantic-type-inference/src/solver.rs` was doing strict label matching, requiring `actual_label == expected_label`. This was too restrictive for enum constructors called via shorthand syntax. The fix modifies the label matching logic to allow unlabeled arguments (`None`) to match any parameter positionally, while still requiring that explicitly labeled arguments match their expected labels. This brings enum shorthand syntax in line with Kestrel's function calling conventions.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/enums.rs::regression::enum_shorthand_with_unlabeled_arguments`

---

### ~~`Self` doesn't work for calling static methods~~ (Fixed)

**Error:** `undefined name 'Self'`

**Cause:** Using `Self` to call a static method from within the same struct failed because `Self` was only handled in type contexts (like type annotations), not in value expressions (like method calls). When resolving `Self.nextPowerOfTwo(capacity)`, the compiler would try to look up `Self` as a value path and fail to find it.

**Root cause and fix:** The issue was in `lib/kestrel-semantic-tree-binder/src/body_resolver/paths.rs`. The `resolve_path_expression` function only handled `self` (lowercase) for instance method contexts but didn't handle `Self` (uppercase) for referring to the containing type. The fix added a check for `Self` that:
1. Detects when `Self` is used in a value path expression
2. Calls `get_containing_type_for_self()` to get the containing struct/enum type
3. Creates a `TypeRef` expression that can be used to access static methods
4. Resolves the rest of the path (like `.nextPowerOfTwo`) as member accesses on that type

This works for structs, enums, extensions, and initializers, properly handling generic type parameters.

**Status:** Fixed - `Self` can now be used to call static methods from within the same type.

**Regression tests:**
- `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::self_static_method_call`
- `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::self_static_method_call_non_generic`

---

## Control Flow Issues

### ~~Match in init doesn't prove field initialization~~ (Fixed)

**Error:** `initializer does not initialize all fields: 'ptr'`

**Cause:** When using match expressions in initializers, the compiler wasn't properly merging the initialization states from all match arms. The `InitializerVerificationAnalyzer` analyzed each arm separately but discarded the results instead of merging them like it does for `if-else` expressions. This meant that even when:
- One arm initialized the field and another diverged (panicked), OR
- All arms initialized the field

The compiler would still report that the field wasn't initialized.

**Status:** Fixed - match expressions now correctly merge initialization states from all arms.

**Root cause and fix:**

The issue was in `lib/kestrel-semantic-analyzers/src/analyzers/initializer_verification/mod.rs`. The `analyze_expression` function's handler for `ExprKind::Match` analyzed each arm but didn't merge their final states:

```rust
// Before (buggy):
ExprKind::Match { scrutinee, arms } => {
    state = analyze_expression(scrutinee, state, false, ctx);
    for arm in arms {
        let mut arm_state = state.clone();
        // ... analyze arm ...
        // BUG: arm_state is discarded here!
    }
}
```

The fix collects the final state from each arm and merges them using the existing `InitState::merge` method (which already handles diverged states correctly):

```rust
// After (fixed):
ExprKind::Match { scrutinee, arms } => {
    state = analyze_expression(scrutinee, state, false, ctx);
    let mut arm_states: Vec<InitState> = Vec::new();
    for arm in arms {
        let mut arm_state = state.clone();
        // ... analyze arm ...
        arm_states.push(arm_state);
    }
    // Merge all arm states (similar to if-else merging)
    if !arm_states.is_empty() {
        let mut iter = arm_states.into_iter();
        let mut merged = iter.next().unwrap();
        for arm_state in iter {
            merged = merged.merge(arm_state);
        }
        state = merged;
    }
}
```

The `InitState::merge` method already had the correct logic to handle diverging branches: if both branches diverge, only fields initialized in BOTH are considered initialized; if one diverges, use the non-diverging branch's state.

**Regression tests:**
- `lib/kestrel-test-suite/tests/validation/initializers.rs::match_expressions::match_with_diverging_branch`
- `lib/kestrel-test-suite/tests/validation/initializers.rs::match_expressions::match_all_arms_initialize`
- `lib/kestrel-test-suite/tests/validation/initializers.rs::match_expressions::match_not_all_arms_initialize` (negative test)

---

### ~~`while true` with unreachable code after loop causes issues~~ (Fixed)

**Error:** `undefined name 'None'` (on unreachable code)

**Cause:** When `.None` appeared on a new line after a `while` expression, the parser treated it as a postfix member access on the `while` expression (which returns unit type `()`), rather than as an implicit member expression. This caused "undefined name 'None'" because unit has no members:

```kestrel
while true {
    if condition { return .Some(x) }
    if done { return .None }
}
.None  // Was parsed as: (while ...).None
```

**Status:** Fixed - the parser now requires postfix member access dots to immediately follow the receiver expression (same line), preventing newline-separated expressions from being parsed as member chains.

**Root cause and fix:**

The parser's postfix member access combinator was using `skip_trivia()` before matching the dot token, which allowed it to skip newlines and treat `.None` on a new line as a member access on the preceding expression.

The fix (in `lib/kestrel-parser/src/expr/mod.rs`) removed the `skip_trivia()` call before the dot in the postfix `member_access` parser. Now the dot must immediately follow the receiver expression without intervening trivia (newlines, comments). This preserves method chaining on the same line (`obj.method().property`) while preventing cross-line chaining that would be ambiguous with implicit member syntax.

**Regression test:** `lib/kestrel-test-suite/tests/expressions/loops.rs::regression::implicit_member_after_while_not_parsed_as_member_access`

---

## Member Access Issues

### ~~Computed property `.raw` not accessible on local variables~~ (Fixed)

**Error:** `member not found: 'raw' on type 'Int64'` / `undefined name 'raw'`

**Cause:** Single-line computed properties work when accessed on struct fields but fail on local variables:
```kestrel
let byteCount: Int64 = copyCount * elementSize;
memcpy(..., byteCount.raw);  // Error: member not found
```

**Status:** Fixed - computed properties now work correctly on local variables.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::computed_property_on_local_variable`

**Root cause and fix:** Upon investigation, this issue appears to have been resolved by existing fixes in the member access resolution code. The `resolve_member_access` function in `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs` correctly handles computed properties via `ComputedMemberAccessBehavior` for both struct fields and local variables. Testing confirms that both multi-line and single-line computed property syntaxes work correctly on local variables.

---

### ~~Subscript parameters not bound in getter/setter body~~ (Fixed)

**Error:** `undefined name 'index'`

**Cause:** Subscript parameter names were not accessible inside the getter/setter body because both `SubscriptBinder` and `GetterBinder`/`SetterBinder` were binding the getter/setter bodies. The `SubscriptBinder` correctly added subscript parameters to the local scope, but then `GetterBinder`/`SetterBinder` would create a new `LocalScope` without the parameters and overwrite the `ExecutableBehavior`, causing the parameters to be lost.

**Root cause:** The binder registry invoked both the `SubscriptBinder` (for `SyntaxKind::SubscriptDeclaration`) and the `GetterBinder`/`SetterBinder` (for `SyntaxKind::GetterClause`/`SyntaxKind::SetterClause`) on the same getter/setter symbols. Since binders are invoked based on syntax node kind, and subscripts contain getter/setter clauses, both binders would run.

**Fix:** Modified `GetterBinder::bind_body` and `SetterBinder::bind_body` to check if their parent symbol is a `Subscript`. If so, they skip body binding and delegate to `SubscriptBinder` which properly handles subscript parameters.

**Status:** Fixed - subscript parameters are now accessible in both getter and setter bodies.

**Regression tests:**
- `lib/kestrel-test-suite/tests/declarations/subscripts.rs::regression::subscript_parameter_accessible_in_getter`
- `lib/kestrel-test-suite/tests/declarations/subscripts.rs::regression::subscript_parameter_accessible_in_setter`
- `lib/kestrel-test-suite/tests/declarations/subscripts.rs::regression::subscript_multiple_parameters`

---

## Tuple Issues

### ~~Inline tuple in `.Some()` fails type inference~~ (Fixed)

**Error:** `type mismatch: expected 'T', found '(I64, I64)'`

**Cause:** Creating a tuple inline inside `.Some()` and passing it to a generic function failed type inference:
```kestrel
func process[T](opt: Option[T]) -> I64 { ... }
process(.Some((5, 10)))  // Error: type mismatch
```

The issue had two root causes:
1. The `infer_from_type` function in `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs` had no case for `TyKind::Enum` to recursively match enum type arguments during type inference
2. When generic functions were called without explicit type parameters, any type parameters that couldn't be immediately inferred were left as `TypeParameter` types instead of being replaced with fresh `Infer` types

**Status:** Fixed - enum type arguments now participate in type inference, and all type parameters get `Infer` substitutions when not explicitly provided.

**Regression test:** `lib/kestrel-test-suite/tests/declarations/enums.rs::regression::implicit_member_with_tuple_in_generic_function`

**Root cause and fix:**

When a generic function like `process[T](opt: Option[T])` is called with an argument like `.Some((5, 10))`:
1. The implicit member `.Some((5, 10))` initially has type `Option[Infer]` (the tuple type isn't resolved yet)
2. The generic function needs to infer `T` from the argument type
3. During binding, `infer_from_type` tries to match parameter type `Option[T]` against argument type `Option[Infer]`
4. But the function had no case for enums, so it couldn't recursively match type arguments
5. Even if inference succeeded later, type parameters without inferred values stayed as `TypeParameter` instead of becoming `Infer`

The fix was implemented in two parts:

1. **Added enum case to `infer_from_type`** (`lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs` lines 723-743): Added a `TyKind::Enum` match arm that works like the existing `TyKind::Struct` case - it recursively matches enum type arguments between parameter and argument types.

2. **Ensured complete type parameter substitutions** (`lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs` lines 901-917 and 973-983): Modified generic function call resolution to create fresh `Infer` types for any type parameters that couldn't be inferred from arguments, and insert them into the substitutions map. This ensures all type parameters are replaced (either with concrete types or with `Infer` variables that the solver can unify).

---

## Module System Issues

### ~~Module-level `public let` not supported~~ (Fixed)

**Error:** Parse error - `found 'Equals' expected something else`

**Cause:** Field declarations (which include module-level `let` and `var`) did not support initializer expressions (`= value`). The `FieldDeclarationData` struct lacked an `initializer` field, and the parser didn't parse the `= expr` syntax after the type annotation.

**Status:** Fixed - module-level constants with initializers are now supported:
```kestrel
public let STDIN: lang.i64 = 0   // Now works!
public let STDOUT: lang.i64 = 1  // Now works!
```

**Regression test:** `lib/kestrel-test-suite/tests/declarations/structs.rs::regression::module_level_let_with_initializer`

**Root cause and fix:**

The parser's `field_declaration_parser_internal` function didn't support parsing initializers after the type annotation. Field declarations could have computed property bodies (for getters/setters) but couldn't have simple value initializers.

The fix was implemented in three parts:

1. **Data structure** (`lib/kestrel-parser/src/common/data.rs`): Added an `initializer: Option<(Span, ExprVariant)>` field to `FieldDeclarationData` to store the equals sign span and initializer expression.

2. **Parser** (`lib/kestrel-parser/src/common/parsers.rs`): Updated `field_declaration_parser_internal` to optionally parse `= expr` after the type annotation and before the optional semicolon.

3. **Emitter** (`lib/kestrel-parser/src/common/emitters.rs`): Updated `emit_field_declaration` to emit the equals token and initializer expression when present.

This allows module-level constants to be declared with initial values, matching the syntax used for local variable declarations with initializers.
