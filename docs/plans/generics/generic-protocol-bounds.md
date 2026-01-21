# Generic Protocol Bounds Implementation Plan

## Summary

Enable `where T: Protocol[SomeType]` syntax in where clauses. Currently rejected with "generic protocol bounds are not yet supported".

## Example Use Case

```kestrel
protocol Converter[Target] {
    func convert(self) -> Target
}

func useConverter[T](val: T) -> lang.i64 where T: Converter[lang.i64] {
    val.convert()  // Should return lang.i64
}
```

---

## Changes Required

### 1. Remove Rejection in `utils.rs` (~507-516)

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs`

**Current:**
```rust
if !substitutions.is_empty() {
    // This is a generic protocol bound like Container[E]
    let protocol_name = symbol.metadata().name().value.clone();
    let error = UnsupportedGenericProtocolBoundError { ... };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
    return false; // Filter out this bound
}
```

**Change:** Remove this `if` block entirely.

---

### 2. Remove Rejection in `members.rs` - `resolve_constrained_member_access` (~479-486)

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

**Current:**
```rust
if !proto.type_parameters().is_empty() {
    let error = UnsupportedGenericProtocolBoundError { ... };
    ctx.diagnostics.add_diagnostic(error.into_diagnostic());
    return Expression::error(full_span);
}
```

**Change:** Remove this `if` block entirely.

---

### 3. Remove Rejection in `members.rs` - `resolve_constrained_member_call` (~979-986)

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

Same pattern as above, remove the check.

---

### 4. Capture Substitutions When Processing Bounds

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

**In `resolve_constrained_member_call` (line ~974):**

```rust
// Current:
if let TyKind::Protocol { symbol: proto, .. } = bound.kind() {

// Change to:
if let TyKind::Protocol { symbol: proto, substitutions } = bound.kind() {
```

Then pass `substitutions` to `collect_protocol_methods`.

---

### 5. Modify `collect_protocol_methods` Signature

**Current:**
```rust
fn collect_protocol_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    receiver_ty: &Ty,
    candidates: &mut Vec<ConstrainedMethodCandidate>,
    ctx: &BodyResolutionContext,
)
```

**Change to:**
```rust
fn collect_protocol_methods(
    protocol: &Arc<ProtocolSymbol>,
    method_name: &str,
    receiver_ty: &Ty,
    protocol_substitutions: &Substitutions,  // NEW
    candidates: &mut Vec<ConstrainedMethodCandidate>,
    ctx: &BodyResolutionContext,
)
```

---

### 6. Apply Protocol Substitutions to Callable

**In `collect_protocol_methods` (around line 1114-1115):**

```rust
// Current:
let substituted_callable = substitute_callable_self(&callable, receiver_ty);

// Change to:
let substituted_callable = substitute_callable_self(&callable, receiver_ty);
let substituted_callable = substitute_callable(&substituted_callable, protocol_substitutions);
```

**New function needed:**
```rust
/// Substitute type parameters in a CallableBehavior.
fn substitute_callable(callable: &CallableBehavior, substitutions: &Substitutions) -> CallableBehavior {
    use kestrel_semantic_tree::behavior::callable::CallableParameter;

    let new_params: Vec<CallableParameter> = callable
        .parameters()
        .iter()
        .map(|p| CallableParameter {
            access_mode: p.access_mode,
            ty: substitute_type(&p.ty, substitutions),
            label: p.label.clone(),
            bind_name: p.bind_name.clone(),
        })
        .collect();

    let new_return = substitute_type(callable.return_type(), substitutions);

    match callable.receiver() {
        Some(receiver_kind) => CallableBehavior::with_receiver(
            new_params,
            new_return,
            receiver_kind,
            callable.span().clone(),
        ),
        None => CallableBehavior::new(new_params, new_return, callable.span().clone()),
    }
}
```

---

### 7. Handle Inheritance - Compose Substitutions

**In `collect_protocol_methods` (around line 1152-1156):**

```rust
// Current:
for parent_proto_ty in conformances.conformances() {
    if let TyKind::Protocol { symbol: parent, .. } = parent_proto_ty.kind() {
        collect_protocol_methods(parent, method_name, receiver_ty, candidates, ctx);
    }
}

// Change to:
for parent_proto_ty in conformances.conformances() {
    if let TyKind::Protocol { symbol: parent, substitutions: parent_subs } = parent_proto_ty.kind() {
        // Apply our substitutions to the parent's type arguments
        let composed_subs = compose_substitutions(protocol_substitutions, parent_subs);
        collect_protocol_methods(parent, method_name, receiver_ty, &composed_subs, candidates, ctx);
    }
}
```

**New function needed:**
```rust
/// Compose substitutions: apply outer substitutions to inner substitution values.
fn compose_substitutions(outer: &Substitutions, inner: &Substitutions) -> Substitutions {
    let mut result = Substitutions::new();
    for (id, ty) in inner.iter() {
        result.insert(*id, substitute_type(ty, outer));
    }
    result
}
```

---

### 8. Update Ambiguity Detection

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

**Issue:** Currently uses protocol name for ambiguity detection. With `T: Converter[i64], T: Converter[String]`, both have name "Converter" but should be considered different (or same protocol with different type args = ambiguous).

**In `resolve_constrained_member_call` (around line 1032-1036):**

The current deduplication uses `protocol_name`. Need to also consider substitutions.

**Option A (simpler):** Treat same protocol with different substitutions as ambiguous (error).
**Option B (more complex):** Treat them as distinct method sources.

Recommend Option A - if you have `T: Converter[i64]` and `T: Converter[String]`, calling `t.convert()` is genuinely ambiguous about which return type you want.

**Change:** Add substitutions to `ConstrainedMethodCandidate` struct and include in uniqueness check:

```rust
struct ConstrainedMethodCandidate {
    method: Arc<dyn Symbol<KestrelLanguage>>,
    callable: CallableBehavior,
    protocol_name: String,
    protocol_substitutions: Substitutions,  // NEW
    definition_span: Span,
}
```

Update deduplication to compare both name AND substitutions.

---

### 9. Update Error Messages

**File:** `lib/kestrel-semantic-tree-binder/src/diagnostics/member_access.rs`

Update error messages to show substituted protocol names like `Converter[lang.i64]` instead of just `Converter`.

**Add helper function:**
```rust
fn format_protocol_with_substitutions(name: &str, subs: &Substitutions) -> String {
    if subs.is_empty() {
        name.to_string()
    } else {
        let args: Vec<String> = subs.values().map(|ty| ty.to_string()).collect();
        format!("{}[{}]", name, args.join(", "))
    }
}
```

---

## Tests

**Location:** `lib/kestrel-test-suite/tests/types/static_type_param.rs`

Add new module `generic_protocol_bounds` after the existing `edge_cases` module.

### Test Cases

```rust
mod generic_protocol_bounds {
    use super::*;

    #[test]
    fn basic_generic_protocol_bound() {
        // where T: Converter[lang.i64] - basic case
        Test::new(
            r#"module Test
            protocol Converter[Target] {
                func convert(self) -> Target
            }
            func useConverter[T](val: T) -> lang.i64 where T: Converter[lang.i64] {
                val.convert()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_protocol_bound_with_type_parameter() {
        // where T: Container[E] - protocol arg is another type param
        Test::new(
            r#"module Test
            protocol Container[E] {
                func first(self) -> E
            }
            func getFirst[T, E](c: T) -> E where T: Container[E] {
                c.first()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_protocol_bound_static_method() {
        // Static method on generic protocol bound
        Test::new(
            r#"module Test
            protocol Factory[T] {
                static func create() -> T
            }
            func makeWidget[F]() -> lang.i64 where F: Factory[lang.i64] {
                F.create()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_protocol_bound_init() {
        // Init on generic protocol bound
        Test::new(
            r#"module Test
            protocol Buildable[T] {
                init(value: T)
            }
            func build[B](v: lang.i64) -> B where B: Buildable[lang.i64] {
                B(v)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_protocol_bound_self_and_type_param() {
        // Method uses both Self and protocol type param
        Test::new(
            r#"module Test
            protocol Transformer[Output] {
                func transform(self) -> Output
                func chain(self, other: Self) -> Output
            }
            func apply[T](a: T, b: T) -> lang.i64 where T: Transformer[lang.i64] {
                a.chain(b)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_protocol_inheritance() {
        // Child protocol inherits from generic parent
        Test::new(
            r#"module Test
            protocol Converter[T] {
                func convert(self) -> T
            }
            protocol IntConverter: Converter[lang.i64] {
                func convertTwice(self) -> lang.i64
            }
            func useIntConverter[T](val: T) -> lang.i64 where T: IntConverter {
                val.convert()
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn generic_protocol_bound_multiple_type_params() {
        // Protocol with multiple type parameters
        Test::new(
            r#"module Test
            protocol BiConverter[From, To] {
                func convert(self, input: From) -> To
            }
            func transform[T](c: T, input: lang.str) -> lang.i64 where T: BiConverter[lang.str, lang.i64] {
                c.convert(input)
            }
        "#,
        )
        .expect(Compiles);
    }

    #[test]
    fn ambiguous_same_generic_protocol_different_args() {
        // T: Converter[i64] and T: Converter[str] - ambiguous
        Test::new(
            r#"module Test
            protocol Converter[Target] {
                func convert(self) -> Target
            }
            func ambiguous[T](val: T) where T: Converter[lang.i64], T: Converter[lang.str] {
                val.convert()
            }
        "#,
        )
        .expect(HasError("ambiguous"));
    }

    #[test]
    fn recursive_type_param_in_bound() {
        // where T: Comparable[T] - common pattern
        Test::new(
            r#"module Test
            protocol Comparable[Other] {
                func compare(self, other: Other) -> lang.i64
            }
            func compareToSelf[T](a: T, b: T) -> lang.i64 where T: Comparable[T] {
                a.compare(b)
            }
        "#,
        )
        .expect(Compiles);
    }
}
```

### Update Existing Test

**Change the existing test at line ~547-560:**

```rust
#[test]
fn generic_protocol_bound() {
    // T: Container[E] with generic protocol - NOW SUPPORTED
    Test::new(
        r#"module Test
        protocol Container[E] {
            static func empty() -> Self
        }
        func makeEmpty[T, E]() -> T where T: Container[E] {
            T.empty()
        }
    "#,
    )
    .expect(Compiles);  // Changed from HasError("generic protocol bounds")
}
```

---

## Potential Issues / Edge Cases

1. **FlattenedProtocolBehavior** stores methods without substitutions. This is OK because we apply substitutions at lookup time from the bound's `TyKind::Protocol { substitutions, .. }`.

2. **Inference from conformance** (inferring `E` from `MyType: Container[Int]`) is out of scope for this change.

3. **Generic methods inside generic protocols** - two layers of substitution need to compose correctly. The method's own type params are inferred separately from the protocol's.

---

## Files Modified

1. `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs`
2. `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`
3. `lib/kestrel-semantic-tree-binder/src/diagnostics/member_access.rs` (error messages)
4. `lib/kestrel-test-suite/tests/types/static_type_param.rs` (tests)

## Removed Diagnostic

- `UnsupportedGenericProtocolBoundError` - can be removed entirely or kept for other future use
