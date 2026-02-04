# Monomorphization Witness Fix Plan

**Status**: Planning Phase
**Created**: 2026-02-04
**Goal**: Fix witness resolution issues preventing iterator tests from passing

---

## Executive Summary

Five interconnected issues prevent proper witness resolution during monomorphization:

1. **Protocol type lowering returns `<error>`** - Breaking protocol extension witness generation
2. **Witness bindings point to declarations** - Not pointing to actual implementations
3. **Self type propagation bugs** - Wrong Self types bleeding across contexts
4. **Operators use direct calls** - Should use witness calls for proper resolution
5. **Enums lack witness generation** - Missing call to `generate_witnesses_for_enum()`

**Minimal fix for iterator tests**: Issues #1, #4, and #5 (estimated 1-2 days)
**Complete fix**: All 5 issues (estimated 3-4 days)

---

## Issue Priority & Dependencies

### Priority 1: Critical Path Issues (Blocking Iterator Tests)

**Issue #4**: Operators desugar to direct calls
**Issue #1**: Protocol type lowering returns `<error>`
**Issue #5**: Enums don't get witness generation

These three must be fixed for iterator tests to pass.

### Priority 2: Correctness Issues (Should Fix)

**Issue #2**: Witness bindings point to declarations
**Issue #3**: Self type propagation bugs

These cause incorrect behavior but may not immediately break tests.

### Dependency Graph

```
Issue #1 (Protocol lowering)
   ↓ blocks
Issue #4 (Operator desugaring) ← depends on proper protocol handling
   ↓
Issue #5 (Enum witnesses) ← independent but needed for tests
   ↓
Issues #2 & #3 ← can be fixed afterward for correctness
```

**Recommendation**: Fix in order 1 → 4 → 5 → 2 → 3

---

## Detailed Fix Plans

---

## Issue #1: Protocol Type Lowering Returns `<error>`

### Problem

**File**: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-execution-graph-lowering/src/ty.rs:100-107`

When `generate_witnesses_for_extension()` processes `extend Comparable: Less[Self]`, it calls `lower_type()` with `target_ty = Ty::Protocol(Comparable)`. This hits:

```rust
TyKind::Protocol { symbol, .. } => {
    ctx.emit_error(LoweringError::unsupported_type(...));
    ctx.mir.ty_error()  // Returns <error>
}
```

This error type becomes the `implementing_type` for all derived witnesses from that extension, creating:

```
witness <error>: std.core.Less { }
witness <error>: std.core.LessOrEqual { }
```

### Root Cause

Protocol extensions like `extend Comparable: Less[Self]` should generate **conditional witnesses** (witnesses that apply when a type conforms to the base protocol), not concrete witnesses. The current code tries to treat the protocol as a concrete type.

### Solution

**Option A: Skip protocol extension witnesses during lowering** (Minimal Fix)
- Protocol extensions should NOT generate witnesses directly
- Instead, when a concrete type conforms to `Comparable`, generate derived witnesses
- The infrastructure already exists in `generate_derived_witnesses_for_protocol_extensions()`

**Option B: Generate conditional witness patterns** (Complete Fix)
- Introduce `MirTy::ConditionalWitness` or similar
- Store witness patterns like "for all T where T: Comparable, T: Less"
- Requires monomorphization changes to handle conditional witnesses

**Recommendation**: Option A (skip during lowering)

### Implementation Plan

**Files to modify**:
1. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-execution-graph-lowering/src/lowerer/witness.rs`

**Changes**:

```rust
// In generate_witnesses_for_extension() around line 124
pub fn generate_witnesses_for_extension(
    ctx: &mut LoweringContext,
    extension_symbol: &Arc<ExtensionSymbol>,
) {
    let Some(target_ty) = extension_symbol.target_type() else {
        return;
    };

    // Protocol extensions don't generate witnesses directly.
    // When `extend Comparable: Less[Self]` is defined, no witness is created here.
    // Instead, when a concrete type `struct X: Comparable` is processed,
    // the derived witness `X: Less` is generated through
    // `generate_derived_witnesses_for_protocol_extensions()`.
    if matches!(target_ty.kind(), TyKind::Protocol { .. }) {
        return;  // ← Already has this early return!
    }

    // Rest of function handles concrete type extensions...
}
```

**Wait, this check already exists!** The bug is that it's checking too late or the check isn't working.

Let me trace the actual bug:

Looking at line 124 in witness.rs, the early return for protocol extensions IS there. So why are broken witnesses being generated?

**Actual Root Cause**: The derived witness generation in `generate_derived_witnesses_for_protocol_extensions()` (line 586) is being called, but it's using the wrong implementing_type. When called from `generate_witnesses_for_struct()`, the `implementing_type` should be the struct type, not the protocol type.

**Real Fix**: The issue is in `generate_witnesses_for_struct()` or how it calls the derived witness function.

Actually, re-reading the bug report: The `<error>` witnesses are created by line 71 in the OLD code path before the early return was added. The fix is already in place (line 124 early return).

**Status**: This may already be fixed in current code. Need to verify by checking if the early return at line 124 is being executed.

### Testing

```bash
cargo run -- check lang/std2/**/*.ks --xgraph | grep "witness <error>"
```

Expected: No `<error>` witnesses should be generated.

**Complexity**: Simple (if not already fixed)
**Risk**: Low - Early return is safe
**Time Estimate**: 1 hour to verify and test

---

## Issue #4: Operators Desugar to Direct Calls Instead of Witness Calls

### Problem

**Flow**:
1. User writes `a < b`
2. Desugaring (operators.rs:321-378) → `a.lessThan(b)` as deferred method call
3. Type resolution finds `Comparable.lessThan` from protocol extension
4. Lowering (expr.rs) emits `Callee::Direct` for concrete types
5. During monomorphization, Self type gets confused

**Example**:
```
ArrayIterator[T=File].next() contains:
    self.remaining > 0  (Int64 comparison)
    ↓ lowered as
    call Comparable.greaterThan(ref self.remaining, ref zero)
    ↓ during monomorphization
    Self type incorrectly picks up File from enclosing context
    ↓ results in
    FunctionInstantiation { func: Comparable.greaterThan, self_type: Some(File) }
    ↓ inside greaterThan
    self.compare(other) needs Comparable for File witness
    ↓ fails
    "no witness found: protocol Comparable for type File"
```

### Root Cause

Operators should desugar to protocol method calls (witness-based), not to extension method calls (direct). When type resolution finds `Comparable.lessThan`, it should record that this is dispatched through the `Less` protocol, not directly.

### Solution

**Two-part fix**:

**Part A: Type Resolution** - Record the protocol being dispatched through
**Part B: Lowering** - Emit witness calls for protocol-dispatched methods

### Implementation Plan

**Files to modify**:

1. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-semantic-tree/src/expr.rs`
   - Add field to `MethodRef` to track dispatch protocol

2. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs:321-378`
   - Modify `desugar_binary_op()` to record dispatch protocol

3. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-semantic-model/src/type_oracle.rs:194-225`
   - When resolving methods from protocol extensions, return the protocol being used

4. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-execution-graph-lowering/src/expr.rs`
   - Use dispatch protocol to emit `Callee::Witness` instead of `Callee::Direct`

**Detailed Changes**:

**Step 1: Add protocol tracking to Expression**

```rust
// In /lib/kestrel-semantic-tree/src/expr.rs
pub enum Expression {
    // ... existing variants ...

    MethodRef {
        receiver: Box<Expression>,
        candidates: Vec<SymbolId>,
        name: String,
        span: Span,
        // NEW: Track which protocol this method is dispatched through
        dispatch_protocol: Option<(SymbolId, String)>, // (protocol_id, method_name)
    },
}
```

**Step 2: Record protocol during operator desugaring**

```rust
// In /lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs
fn desugar_binary_op(...) -> Expression {
    // ... existing checks ...

    let method_name = op.method_name();

    // Try to use builtin registry
    if let Some(feature) = op.method_feature()
        && let Some(method_id) = ctx.model.builtin_registry().method(feature)
    {
        // NEW: Get the protocol that defines this method
        let dispatch_protocol = if let Some(method_sym) = ctx.model.query(SymbolFor { id: method_id }) {
            if let Some(parent) = method_sym.metadata().parent() {
                if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                    Some((parent.metadata().id(), method_name.to_string()))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mut method_ref = Expression::method_ref(
            lhs,
            vec![method_id],
            method_name.to_string(),
            full_span.clone(),
        );

        // Set dispatch_protocol on the MethodRef
        if let Expression::MethodRef { ref mut dispatch_protocol: dp, .. } = method_ref {
            *dp = dispatch_protocol;
        }

        return Expression::call(method_ref, vec![arg], result_ty, full_span);
    }

    // ... fallback ...
}
```

**Step 3: Use protocol info during lowering**

```rust
// In /lib/kestrel-execution-graph-lowering/src/expr.rs
// Around the method call lowering (need to find exact location)

fn lower_method_call(...) {
    // ... existing code ...

    // Check if this method should be dispatched through a protocol
    if let Some((protocol_id, method_name)) = &method_ref.dispatch_protocol {
        // Get protocol's qualified name
        let protocol_symbol = ctx.model.query(SymbolFor { id: *protocol_id }).unwrap();
        let protocol_qname = qualified_name_for_symbol(ctx, &protocol_symbol);

        // Get receiver type
        let receiver_ty = lower_type(ctx, &receiver.ty);

        // Emit witness call
        let callee = Callee::Witness {
            protocol: protocol_qname,
            method: method_name.clone(),
            for_type: receiver_ty,
            method_type_args: vec![], // Add if method has type params
        };

        // ... emit call with callee ...
    } else {
        // Use direct call as before
        // ...
    }
}
```

**Alternative Simpler Approach**: Instead of tracking dispatch protocol, check at lowering time if the resolved method is in a protocol extension:

```rust
// In expr.rs during method lowering
fn should_use_witness_dispatch(method_symbol: &dyn Symbol, receiver_ty: &Ty) -> Option<...> {
    // If method is in an extension on a protocol, use witness dispatch
    if let Some(parent) = method_symbol.metadata().parent() {
        if parent.metadata().kind() == KestrelSymbolKind::Extension {
            // Get extension's target
            if let Ok(ext) = parent.clone().downcast_arc::<ExtensionSymbol>() {
                if let Some(target_ty) = ext.target_type() {
                    if matches!(target_ty.kind(), TyKind::Protocol { .. }) {
                        // This is a protocol extension method
                        // Find which protocol it conforms to (from the extension's conformances)
                        // ...
                        return Some((protocol_id, method_name));
                    }
                }
            }
        }
    }
    None
}
```

**Recommendation**: Use the simpler approach (check at lowering time) to avoid changing Expression representation.

### Testing

```kestrel
// Test file: test_operator_witness_dispatch.ks
module Test

struct MyInt {
    value: Int64
}

extend MyInt: Comparable {
    func compare(other: MyInt) -> Ordering {
        self.value.compare(other.value)
    }
}

func test() {
    let a = MyInt(value: 5)
    let b = MyInt(value: 3)
    let result = a < b  // Should use witness dispatch
}
```

Check MIR output:
```bash
cargo run -- check test_operator_witness_dispatch.ks --xgraph | grep "call.*lessThan"
```

Expected: `call witness_method std.core.Less.lessThan for Test.MyInt`
Not: `call std.core.Comparable.lessThan`

**Complexity**: Medium-High
**Risk**: Medium - Changes core operator semantics
**Time Estimate**: 6-8 hours

---

## Issue #5: Enums Don't Get Witness Generation

### Problem

**File**: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-execution-graph-lowering/src/lowerer/item.rs:92-119`

Struct handling (lines 46-75) calls `generate_witnesses_for_struct()` after lowering methods.
Enum handling (lines 92-119) never calls witness generation.

The function `generate_witnesses_for_enum()` already exists in witness.rs (line 70) and is already exported from mod.rs (line 20), but it's never called.

### Solution

Add the call to `generate_witnesses_for_enum()` in the enum handling code.

### Implementation Plan

**Files to modify**:
1. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-execution-graph-lowering/src/lowerer/item.rs`

**Changes**:

```rust
// Around line 115-118
KestrelSymbolKind::Enum => {
    if let Ok(enum_symbol) = symbol.clone().downcast_arc::<EnumSymbol>() {
        lower_enum(ctx, &enum_symbol);

        // Also lower methods, computed properties, and subscripts within the enum
        for child in symbol.metadata().children() {
            let child_kind = child.metadata().kind();
            if child_kind == KestrelSymbolKind::Function {
                lower_item(ctx, &child);
            } else if child_kind == KestrelSymbolKind::Field
                || child_kind == KestrelSymbolKind::Subscript
            {
                // Lower getters and setters within fields (computed properties) and subscripts
                for field_child in child.metadata().children() {
                    let fc_kind = field_child.metadata().kind();
                    if fc_kind == KestrelSymbolKind::Getter
                        || fc_kind == KestrelSymbolKind::Setter
                    {
                        lower_item(ctx, &field_child);
                    }
                }
            }
        }

        // Generate witnesses for protocol conformances
        generate_witnesses_for_enum(ctx, &enum_symbol);  // ← ADD THIS LINE
    }
}
```

### Testing

```kestrel
// Test file: test_enum_witness.ks
module Test

enum Color: Equatable {
    case red
    case green
    case blue

    func equals(other: Color) -> Bool {
        match (self, other) {
            (.red, .red) => true,
            (.green, .green) => true,
            (.blue, .blue) => true,
            _ => false
        }
    }
}

func test() {
    let a = Color.red
    let b = Color.red
    let result = a.equals(b)
}
```

Check MIR output:
```bash
cargo run -- check test_enum_witness.ks --xgraph | grep "witness.*Color"
```

Expected: `witness Test.Color: std.core.Equatable { func equals = Test.Color.equals }`

**Complexity**: Simple
**Risk**: Very Low - Just adding a missing function call
**Time Estimate**: 30 minutes

---

## Issue #2: Witness Bindings Point to Declarations, Not Implementations

### Problem

**File**: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-execution-graph-lowering/src/lowerer/witness.rs:683-726`

When generating derived witnesses from protocol extensions, `bind_methods_from_extension()` looks for methods by name in the extension. For `extend Comparable: Less[Self]`, it finds `lessThan` and binds it to `Comparable.lessThan`.

But `Comparable.lessThan` is just the declaration in the Comparable extension (line 97 of protocols.ks). It's not in a separate `extend Comparable: Less[Self]` - it IS in that extension.

**Current behavior**:
```
witness Int64: std.core.Less {
    func lessThan = std.core.Comparable.lessThan
}
```

**Expected behavior**:
```
witness Int64: std.core.Less {
    func lessThan = std.core.Comparable.lessThan  // This is correct!
}
```

Wait, this might not be a bug. Let me re-read...

The implementation of `lessThan` IS in `extend Comparable: Less[Self]` (lines 96-99 of protocols.ks). So the binding should point to that function symbol, which has the qualified name `std.core.Comparable.lessThan`.

**Actual Issue**: When monomorphization looks up `Comparable.lessThan`, it needs to know that Self should be substituted. The witness binding is correct, but the function instantiation needs to include `self_type`.

Looking at collect.rs line 546-554, when processing a witness call, it creates:
```rust
FunctionInstantiation::with_self_type(
    func_id,
    impl_type_args,
    concrete_for_type,
)
```

This DOES set the self_type! So the witness binding is correct.

**Conclusion**: This might not actually be a bug. The witness binding points to the implementation (which is also the declaration in this case, since protocol extension methods have bodies). The self_type is correctly propagated during monomorphization.

**Status**: NOT A BUG - Witness bindings are correct

---

## Issue #3: Self Type Propagation Bug

### Problem

When processing `ArrayIterator[T=File].next()`, comparisons like `remaining > 0` (Int64) incorrectly pick up `Self=File` from the enclosing context.

**Example**:
```
Processing: ArrayIterator[File].next()
  ↓ contains
  self.remaining > 0  (where remaining: Int64)
  ↓ lowered as (if using direct call)
  call Comparable.greaterThan(remaining, 0)
  ↓ monomorphization infers Self from call context
  Self type from ArrayIterator[File] bleeds into Int64 comparison
  ↓ results in
  FunctionInstantiation {
      func: Comparable.greaterThan,
      self_type: Some(File)  // WRONG! Should be Int64
  }
```

### Root Cause

**Location**: `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs`

The self_type inference in `scan_callee()` (line 474-567) and `infer_self_type_from_call()` (line 802-889) tries to extract Self from arguments, but can inherit Self from the parent substitution.

**Key Issue**: Line 876-878:
```rust
// If we're already in a Self-typed context, reuse it.
if let Some(existing_st) = subst.get_self_type() {
    return Some(existing_st);
}
```

When processing `ArrayIterator[File].next()`, the substitution has `Self=File`. When we hit the Int64 comparison, this code path returns `File` instead of extracting `Int64` from the actual arguments.

### Solution

**Self type boundaries**: Each function call should start with a clean Self type context, not inherit from the parent.

**Fix**: Don't inherit self_type from parent substitution for direct calls to functions that have their own receiver.

### Implementation Plan

**Files to modify**:
1. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs`

**Changes**:

```rust
// Around line 192-210
fn scan_statement(&mut self, stmt: &StatementKind, subst: &Substitution) {
    match stmt {
        StatementKind::Call { callee, args } => {
            // Create a new substitution without self_type for the call
            // (self_type will be inferred from the call's arguments)
            let mut call_subst = subst.clone();
            call_subst.clear_self_type(); // NEW: Clear inherited self_type

            // Now infer self_type from this specific call
            let self_type = self.infer_self_type_from_call(callee, args, &call_subst);
            if let Some(st) = self_type {
                call_subst.set_self_type(st);
            }

            self.scan_callee(callee, &call_subst);
            for arg in args {
                self.scan_value(&arg.value, subst); // Use parent subst for args
            }
        },
        // ... other cases ...
    }
}
```

Wait, this approach has issues. The type parameters from the parent function (like `T` in `ArrayIterator[T]`) need to be preserved in the substitution, but `Self` should not bleed through.

**Better approach**: Distinguish between type parameters and Self type in the substitution.

```rust
// In infer_self_type_from_direct_call around line 876
fn infer_self_type_from_direct_call(...) -> Option<...> {
    // ... existing code ...

    // Check if the parameter type involves Self
    let needs_self = self.type_contains_self(param_ty);

    if !needs_self {
        return None;
    }

    // REMOVE the early return that reuses existing self_type
    // if let Some(existing_st) = subst.get_self_type() {
    //     return Some(existing_st);
    // }

    // Always extract Self from the actual arguments
    let first_arg = &args[0];
    let arg_ty = self.get_value_type(&first_arg.value, subst)?;

    // Apply substitution to resolve any type params (but not Self!)
    // ...
}
```

Actually, the real issue is that `type_contains_self()` at line 892 returns `true` for `Named` types, which is too conservative:

```rust
fn type_contains_self(&self, ty: &MirTy) -> bool {
    match ty {
        MirTy::SelfType => true,
        MirTy::Named { .. } => {
            // Named types could be protocols in protocol extension methods
            // We need to check if this is actually a protocol acting as Self
            // For now, return true for Named types to be conservative
            // TODO: Add proper protocol detection
            true  // ← TOO BROAD!
        },
        // ...
    }
}
```

**Root Fix**: Make `type_contains_self()` more precise. Only return `true` for actual `SelfType`, not all Named types.

But wait, the comment says "Named types could be protocols in protocol extension methods". This is for handling cases like:

```kestrel
protocol Comparable {
    func compare(other: Self) -> Ordering
}
```

Where the parameter might be typed as `Comparable` but actually means `Self`.

**Better Fix**: Use `type_needs_self()` instead of `type_contains_self()`. It's already defined at line 956 and is more precise:

```rust
fn type_needs_self(&self, ty: &MirTy) -> bool {
    match ty {
        MirTy::SelfType => true,
        MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
            self.type_needs_self(self.mir.ty(*inner))
        },
        MirTy::Tuple(elems) => elems.iter().any(|e| self.type_needs_self(self.mir.ty(*e))),
        MirTy::Named { type_args, .. } => type_args
            .iter()
            .any(|a| self.type_needs_self(self.mir.ty(*a))),
        // ... other cases ...
        _ => false,
    }
}
```

This only returns `true` if the type actually contains `MirTy::SelfType`, not for all Named types.

### Implementation Plan

**Files to modify**:
1. `/Users/dino/Documents/Projects/kestrel/lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs`

**Changes**:

```rust
// Around line 869-878
fn infer_self_type_from_direct_call(...) -> Option<...> {
    // Get the function definition
    let func_def = &self.mir.functions[func_id];

    if func_def.params.is_empty() || args.is_empty() {
        return None;
    }

    // Get the first parameter's type
    let first_param = &self.mir.params[func_def.params[0]];
    let param_ty = self.mir.ty(first_param.ty);

    // Check if the parameter type involves Self
    // CHANGE: Use type_needs_self() instead of type_contains_self()
    let needs_self = self.type_needs_self(param_ty);

    if !needs_self {
        return None;
    }

    // REMOVE: Don't inherit self_type from parent context for functions
    // that have a concrete self parameter
    // if let Some(existing_st) = subst.get_self_type() {
    //     return Some(existing_st);
    // }

    // Extract the concrete type from the first argument
    let first_arg = &args[0];
    let arg_ty = self.get_value_type(&first_arg.value, subst)?;

    // Apply substitution to resolve any type params, but extract concrete type
    if let Ok(substituted_ty) = subst.apply_ty_readonly(self.mir, arg_ty) {
        return self.extract_concrete_type_from_arg(substituted_ty);
    }
    None
}
```

Wait, removing the early return could break legitimate cases where we DO want to inherit Self. For example, inside a protocol extension method, `self.someHelper()` should use the same Self.

**More Nuanced Fix**: Only skip the early return for direct calls where the receiver is NOT a SelfType or type parameter:

```rust
// Around line 869-878
fn infer_self_type_from_direct_call(...) -> Option<...> {
    // ... existing code to get param_ty ...

    let needs_self = self.type_needs_self(param_ty);

    if !needs_self {
        return None;
    }

    // Extract the concrete type from the first argument
    let first_arg = &args[0];
    let arg_ty = self.get_value_type(&first_arg.value, subst)?;

    // NEW: Only reuse existing self_type if the argument is actually Self or a type param
    let arg_mir_ty = self.mir.ty(arg_ty);
    let arg_is_abstract = matches!(arg_mir_ty, MirTy::SelfType | MirTy::TypeParam(_));

    if arg_is_abstract {
        // Argument is abstract - inherit Self from context if available
        if let Some(existing_st) = subst.get_self_type() {
            return Some(existing_st);
        }
    }

    // Argument is concrete - extract Self from it
    if let Ok(substituted_ty) = subst.apply_ty_readonly(self.mir, arg_ty) {
        return self.extract_concrete_type_from_arg(substituted_ty);
    }
    None
}
```

### Testing

```kestrel
module Test

struct Container[T] {
    value: T

    func process() {
        let count = 5
        if count > 0 {  // Should use Self=Int64, NOT Self=T
            // ...
        }
    }
}
```

Check monomorphization output to ensure Int64 comparison doesn't pick up `Self=T`.

**Complexity**: Medium
**Risk**: Medium - Changes self_type inference logic
**Time Estimate**: 4-6 hours

---

## Minimal Fix Strategy

**Goal**: Get iterator tests passing with minimum risk.

**Fix Order**:
1. Issue #5 (Enum witnesses) - 30 minutes, very low risk
2. Issue #1 (Protocol lowering) - 1 hour, verify it's already fixed
3. Issue #4 (Operator dispatch) - 6-8 hours, medium risk

**Total Time**: ~8-10 hours of work

Skip Issues #2 and #3 initially:
- Issue #2 is not actually a bug
- Issue #3 may be masked by fixing Issue #4 (witness dispatch)

**Testing Strategy**:
1. After each fix, run: `cargo test -p kestrel-test-suite`
2. Test specific iterator scenarios
3. Check xgraph output for proper witness calls

---

## Complete Fix Strategy

**Goal**: Full correctness of witness resolution.

**Fix Order**:
1. Issue #5 (Enum witnesses) - 30 minutes
2. Issue #1 (Protocol lowering) - 1 hour
3. Issue #4 (Operator dispatch) - 6-8 hours
4. Issue #3 (Self propagation) - 4-6 hours
5. Issue #2 (skip - not a bug)

**Total Time**: ~12-16 hours of work

---

## Risk Assessment

### Issue #5 (Enum Witnesses)
- **Risk**: Very Low
- **Impact if broken**: Enum conformances won't work
- **Rollback**: Easy - just remove the line

### Issue #1 (Protocol Lowering)
- **Risk**: Very Low
- **Impact if broken**: Might create <error> witnesses (but likely already fixed)
- **Rollback**: Easy - revert the early return

### Issue #4 (Operator Dispatch)
- **Risk**: Medium
- **Impact if broken**: All operator behavior could break
- **Rollback**: Moderate - need to revert operator lowering changes
- **Mitigation**: Extensive testing with operators before committing

### Issue #3 (Self Propagation)
- **Risk**: Medium
- **Impact if broken**: Generic functions might not instantiate correctly
- **Rollback**: Easy - revert the inference changes
- **Mitigation**: Add specific tests for self_type boundaries

---

## Testing Plan

### Unit Tests

For each fix, add tests in:
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/execution_graph/protocols.rs`
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/declarations/extensions.rs`

### Integration Tests

Create test files:
1. `test_enum_equatable.ks` - Enum with Equatable conformance
2. `test_operator_witness.ks` - Operators using protocol extensions
3. `test_nested_generics.ks` - Generic function with operators (Self propagation)

### Regression Tests

Run full test suite:
```bash
cargo test
cargo test -p kestrel-test-suite -- --nocapture
```

### Manual Verification

```bash
# Build example with witnesses
cargo run -- build lang/std2/**/*.ks examples/pong.ks

# Check witness generation
cargo run -- check lang/std2/**/*.ks --xgraph > /tmp/xgraph.txt
grep "witness" /tmp/xgraph.txt
grep "call witness_method" /tmp/xgraph.txt
```

---

## Success Criteria

### Minimal Fix
- ✓ Iterator tests pass
- ✓ No `<error>` witnesses in xgraph output
- ✓ Enum witnesses are generated
- ✓ Operators use witness dispatch

### Complete Fix
- ✓ All of Minimal Fix criteria
- ✓ Self type doesn't bleed across function boundaries
- ✓ All existing tests still pass
- ✓ Pong.ks compiles without witness errors

---

## Estimated Timeline

### Minimal Fix (Issues #5, #1, #4)
- Day 1 Morning: Issues #5 and #1 (verify/fix protocol lowering)
- Day 1 Afternoon: Issue #4 part 1 (design operator dispatch)
- Day 2 Morning: Issue #4 part 2 (implement operator dispatch)
- Day 2 Afternoon: Testing and fixes
- **Total**: 1.5-2 days

### Complete Fix (+ Issue #3)
- Day 3 Morning: Issue #3 (self propagation fix)
- Day 3 Afternoon: Testing and integration
- **Total**: 2.5-3 days from start

---

## Rollback Plan

If any fix causes regressions:

1. **Immediate Rollback**:
   ```bash
   git stash
   git checkout [previous-commit]
   cargo test
   ```

2. **Identify Breaking Change**:
   - Run tests on each commit
   - Bisect if needed

3. **Alternative Approach**:
   - For Issue #4: Could use a feature flag to enable witness dispatch
   - For Issue #3: Could add more conservative checks

---

## Open Questions

1. **Issue #1**: Is the protocol lowering already fixed? Need to verify the early return at line 124 is working.

2. **Issue #4**: Should we change operator desugaring globally, or add a flag to control dispatch mode?

3. **Issue #3**: Are there legitimate cases where Self should be inherited across call boundaries? Need to audit protocol extension usages.

4. **Performance**: Will witness dispatch for all operators impact performance? (Likely minimal - witness resolution is at compile time)

---

## Conclusion

**Recommended Approach**: Start with Minimal Fix (Issues #5, #1, #4)

These three issues are independent, low-risk, and directly block iterator tests. Issue #3 (Self propagation) might be fixed as a side effect of Issue #4 (witness dispatch), since witness calls don't need self_type inference from arguments.

**Next Steps**:
1. Verify Issue #1 is already fixed (check xgraph output)
2. Implement Issue #5 (add one line to item.rs)
3. Design and implement Issue #4 (operator witness dispatch)
4. Test thoroughly
5. Evaluate if Issue #3 still needs fixing

**Total Time for Minimal Fix**: 1-2 days
**Total Time for Complete Fix**: 3-4 days
