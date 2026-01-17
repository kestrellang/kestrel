# Monomorphization Witness Bugs

Discovered while investigating build errors with `cargo run -- build lang/std2/**/*.ks lang/io/**/*.ks examples/pong.ks`.

## Bug 1: Broken Blanket Implementation Witnesses

**Location**: MIR lowering of protocol extensions with protocol conformances

**Symptom**: The `extend Comparable: Less[Self], LessOrEqual[Self], Greater[Self], GreaterOrEqual[Self], NotEqual[Self]` in `protocols.ks` creates witnesses with `<error>` as the implementing type:

```
witness <error>: std.core.Less {
}
witness <error>: std.core.LessOrEqual {
}
witness <error>: std.core.Greater {
}
witness <error>: std.core.GreaterOrEqual {
}
witness <error>: std.core.NotEqual {
}
```

**Problems**:
- `<error>` instead of a proper type pattern (should be `Self` or `T where T: Comparable`)
- Empty method bindings (should have `func lessThan = Comparable.lessThan` etc.)

**Impact**: No type has working `Less`, `Greater`, etc. witnesses from the blanket implementation. Only the broken `<error>` witnesses exist.

**Expected**: Witnesses should be generated like:
```
witness Self: std.core.Less where Self: Comparable {
    type Output = Bool
    func lessThan = std.core.Comparable.lessThan
}
```

---

## Bug 2: Missing Enum Witness Registration

**Location**: Witness generation for enum conformances

**Symptom**: `Pong.Player` enum declares `Equatable` conformance and implements `equals()`:

```kestrel
public enum Player: Equatable {
    case player1
    case player2

    public func equals(other: Player) -> Bool {
        match (self, other) {
            (.player1, .player1) => true,
            (.player2, .player2) => true,
            _ => false
        }
    }
}
```

But no witness is generated in MIR:
```
grep "witness.*Pong" xgraph_output.txt
# (no results)
```

**Impact**: `Dictionary[Player, Int64]` fails because `Dictionary.findEntry` calls `entry.key.equals(key)` which requires `Equatable for Player` witness.

**Expected**: A witness should be generated:
```
witness Pong.Player: std.core.Equatable {
    func equals = Pong.Player.equals
}
```

---

## Bug 3: Operator Desugaring Uses Direct Calls Instead of Witness Calls

**Location**: Operator desugaring / MIR lowering

**Symptom**: When user code uses comparison operators like `a < b`, it's lowered to a direct call:
```
call std.core.Comparable.lessThan(ref a, ref b)
```

Instead of a witness call:
```
call witness_method std.core.Less.lessThan for TypeOf(a)(ref a, ref b)
```

**Impact**:
- The `Self` type must be inferred from the argument during monomorphization
- When processing generic functions, `Self` can be incorrectly propagated
- Protocol extension methods like `Comparable.lessThan` get instantiated with wrong `Self` types (e.g., `Self=Array`)

**Example Flow**:
1. `ArrayIterator.next[T=Array[U]]` compares `self.remaining > 0` (Int64 comparison)
2. This is lowered as `call Comparable.greaterThan(ref self.remaining, ref zero)`
3. During monomorphization, self_type inference can pick up wrong types
4. `Comparable.greaterThan` gets instantiated with `Self=Array`
5. Inside, `self.compare(other)` needs `Comparable for Array` witness
6. Witness lookup fails

**Expected**: Operators should desugar to witness calls, allowing proper witness resolution at the call site.

---

## Reproduction

```bash
cargo run -- build lang/std2/**/*.ks lang/io/**/*.ks examples/pong.ks
```

Errors:
```
- no witness found: protocol std.core.Comparable for type std.collections.Array
- no witness found: protocol std.core.Equatable for type Pong.Player
- no witness found: protocol std.core.Comparable for type Pong.Pong
- no witness found: protocol std.core.Comparable for type std.collections.Dictionary
```

## Debug Commands

```bash
# Generate xgraph output
cargo run -- check lang/std2/**/*.ks lang/io/**/*.ks examples/pong.ks --xgraph > /tmp/xgraph_output.txt 2>&1

# Check for broken witnesses
grep "witness <error>" /tmp/xgraph_output.txt

# Check for missing Player witness
grep "witness.*Pong" /tmp/xgraph_output.txt

# Check how operators are lowered
grep "call std.core.Comparable" /tmp/xgraph_output.txt | head -20
```

---

## Root Cause Analysis

### Bug 1 Root Cause

**File**: `lib/kestrel-execution-graph-lowering/src/ty.rs:110-117`

When `generate_witnesses_for_extension()` processes a protocol extension like `extend Comparable: Less[Self]`, it calls `lower_type(ctx, &target_ty)` at `witness.rs:71` where `target_ty` is `Ty::protocol(Comparable)`.

But `lower_type()` cannot handle protocol types:

```rust
TyKind::Protocol { symbol, .. } => {
    // TODO: Protocol types need witness-based handling
    ctx.emit_error(LoweringError::unsupported_type(
        format!("Protocol type '{}'", symbol.metadata().name().value),
        ty.span().clone(),
    ));
    ctx.mir.ty_error()  // <-- Returns <error>!
}
```

This error type becomes the `implementing_type` for all witnesses from that extension, resulting in `witness <error>: std.core.Less { }`.

**Code flow**:
1. `item.rs:146` → `generate_witnesses_for_extension(ctx, &extension_symbol)`
2. `witness.rs:62-64` → Gets target type as `Ty::protocol(Comparable)`
3. `witness.rs:71` → `let implementing_type = lower_type(ctx, &target_ty)`
4. `ty.rs:110-117` → Returns `ctx.mir.ty_error()` for protocol types
5. `witness.rs:261` → Creates witness with error-typed implementing_type

---

### Bug 2 Root Cause

**File**: `lib/kestrel-execution-graph-lowering/src/lowerer/item.rs:88-110`

The enum dispatch arm never calls witness generation. Compare to structs:

**Struct handling (lines 44-72)**:
```rust
KestrelSymbolKind::Struct => {
    lower_struct(ctx, &struct_symbol);
    // ... lower methods ...
    generate_witnesses_for_struct(ctx, &struct_symbol);  // Line 70
}
```

**Enum handling (lines 88-110)**:
```rust
KestrelSymbolKind::Enum => {
    lower_enum(ctx, &enum_symbol);
    // ... lower methods ...
    // NO witness generation call!
}
```

The function `generate_witnesses_for_enum()` doesn't exist, and there's no call to any witness generation function for enums. The infrastructure in `witness.rs` (like `register_extension_type_params`) already handles `TyKind::Enum` at lines 175-194, so most of the support is there - it's just not wired up.

---

### Bug 3 Root Cause

**Multi-stage issue across several files**:

1. **Desugaring** (`lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs:312-340`):
   - `a < b` becomes `Expression::deferred_method_call(lhs, "lessThan", [rhs], ...)`
   - At this point, the receiver type is preserved but no protocol is specified

2. **Type Resolution** (`lib/kestrel-semantic-model/src/type_oracle.rs:194-225`):
   - When resolving `lessThan` for `Int64`, it searches extensions on conforming protocols
   - It returns the first match found, which may be `Comparable.lessThan` from the extension
   - Doesn't track that this should dispatch through the `Less` protocol

3. **Lowering** (`lib/kestrel-execution-graph-lowering/src/expr.rs:2380-2540`):
   - For concrete receiver types, emits `Callee::direct()` instead of `Callee::witness()`
   - The witness call path (line ~2492) only triggers for type parameters, associated types, self type, or protocol types:
     ```rust
     if is_type_param_call || is_assoc_type_call || is_self_type_call || is_protocol_type_call {
         // Emit witness call
     } else {
         // Emit direct call  <-- Int64.lessThan goes here
     }
     ```

4. **Monomorphization** (`lib/kestrel-codegen-cranelift/src/monomorphize/`):
   - Direct calls to `Comparable.lessThan` carry `Self` in their signature
   - `Self` type propagation doesn't work correctly through direct calls
   - Generic instantiation can bind wrong `Self` types (e.g., `Self=Array` when it should be `Self=Int64`)
