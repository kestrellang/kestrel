# Plan: Thunk Generation for Function References

## Problem Statement

When a regular function (thin) is used as a function value (thick), there's a calling convention mismatch:
- **Thick function calls** pass `env_ptr` as the first argument
- **Regular functions** don't expect an `env_ptr` parameter

Currently, the compiler creates a thick struct `{func_ptr, null_env_ptr}` for function references, but when called through a thick pointer, the null env_ptr is passed as the first argument to the function, which doesn't expect it. This causes crashes.

## Solution: Thunk Generation

Generate wrapper functions ("thunks") that bridge the calling convention gap. When a function reference is used as a thick function value, we generate a thunk that:
1. Accepts `(env_ptr, ...args...)`
2. Ignores the `env_ptr`
3. Calls the original function with just `...args...`

## Implementation Steps

### Step 1: Define Thunk Representation in MIR

**File: `lib/kestrel-execution-graph/src/function/mod.rs`**

Add a new `Origin` variant for thunks:
```rust
pub enum Origin {
    // ... existing variants ...
    /// Thunk function generated for a function reference used as a thick callable.
    FunctionThunk {
        /// The original function being wrapped
        original_function: Id<QualifiedName>,
    },
}
```

### Step 2: Create Thunk Generation in Execution Graph Lowering

**File: `lib/kestrel-execution-graph-lowering/src/thunk.rs` (new file)**

Create a module for generating thunks. The thunk generator:
1. Takes a function symbol (regular function, extern function, or witness method)
2. Creates a new MIR function with:
   - Name: `{original_name}.thunk`
   - First parameter: `env: *i8` (unused)
   - Remaining parameters: same as original function
   - Return type: same as original function
3. Body: Call the original function with all args except env, return the result

```rust
pub fn generate_function_thunk(
    ctx: &mut LoweringContext,
    original_func_name: Id<QualifiedName>,
    param_types: Vec<Id<Ty>>,
    return_type: Id<Ty>,
) -> Id<QualifiedName>
```

### Step 3: Track Which Functions Need Thunks

**File: `lib/kestrel-execution-graph-lowering/src/context.rs`**

Add to `LoweringContext`:
```rust
/// Maps original function names to their generated thunk names
thunk_map: HashMap<Id<QualifiedName>, Id<QualifiedName>>,
```

Add methods:
```rust
fn get_or_create_thunk(&mut self, func_name: Id<QualifiedName>, ...) -> Id<QualifiedName>
```

### Step 4: Modify FunctionRef Lowering in expr.rs

**File: `lib/kestrel-execution-graph-lowering/src/expr.rs`**

When lowering a `SymbolRef` or `OverloadedRef` that resolves to a function being used as a value (not directly called), generate a thunk reference instead:

Find the code that creates `Immediate::function_ref()` and:
1. Check if the target type is `TyKind::Function` (being used as a value)
2. If so, get or create a thunk for this function
3. Use `Rvalue::ApplyPartial { func: thunk_name, captures: vec![] }` instead of raw function ref

### Step 5: Handle Extern Functions

Extern functions need special handling because:
- They use C calling convention
- Their thunks need to call them with C calling convention

The thunk itself should:
- Use the standard Kestrel calling convention (thick-compatible)
- Internally call the extern function with C calling convention

### Step 6: Handle Witness Methods

**File: `lib/kestrel-execution-graph-lowering/src/expr.rs`**

When a witness method is used as a value (not directly called):
1. Generate a thunk that calls the witness method
2. The thunk accepts `(env_ptr, ...args...)`
3. The thunk body calls `witness_method Protocol.method for Type(...args...)`

### Step 7: Modify Codegen for Thunks

**File: `lib/kestrel-codegen-cranelift/src/rvalue.rs`**

The `ImmediateKind::FunctionRef` and `ImmediateKind::WitnessMethod` cases should no longer create thick structs directly. Instead:
- Function references used as values go through `ApplyPartial` with a thunk
- Direct calls continue to use the function reference directly

Remove or simplify the thick-struct creation code in `compile_immediate` for `FunctionRef` and `WitnessMethod`.

### Step 8: Update Monomorphization

**File: `lib/kestrel-codegen-cranelift/src/monomorphize/collect.rs`**

Ensure thunk functions are collected for monomorphization when:
- The thunk is referenced via `ApplyPartial`
- Generic functions with thunks get properly instantiated

### Step 9: Handle Generic Functions

For generic function references like `identity[Int64]`:
1. The thunk needs to be monomorphized with the same type arguments
2. Store type args in the thunk reference

## Key Design Decisions

1. **Thunks are generated at MIR lowering time**, not codegen time. This keeps codegen simpler and allows thunks to go through the same optimization/monomorphization pipeline.

2. **Thunks use ApplyPartial with empty captures**. This creates a proper thick callable struct `{thunk_ptr, null_env}` that's compatible with closure calling convention.

3. **One thunk per unique function signature** could be an optimization, but for simplicity, start with one thunk per function reference.

4. **Extern functions get thunks too**. The thunk uses standard calling convention externally but calls the extern with C convention.

## Files to Modify

1. `lib/kestrel-execution-graph/src/function/mod.rs` - Add `Origin::FunctionThunk`
2. `lib/kestrel-execution-graph-lowering/src/thunk.rs` - New file for thunk generation
3. `lib/kestrel-execution-graph-lowering/src/lib.rs` - Export thunk module
4. `lib/kestrel-execution-graph-lowering/src/context.rs` - Add thunk tracking
5. `lib/kestrel-execution-graph-lowering/src/expr.rs` - Use thunks for function refs as values
6. `lib/kestrel-codegen-cranelift/src/rvalue.rs` - Simplify FunctionRef/WitnessMethod immediate handling

## Testing

After implementation:
1. The `/tmp/func_ptr_test.ks` minimal reproduction should work
2. All 7 failing function pointer tests should pass
3. Closures should continue to work (they already use ApplyPartial correctly)

## Alternative Considered

**VMContext approach**: Add an env parameter to ALL functions. Rejected because:
- Major ABI change affecting all functions
- Incompatible with C calling convention for extern functions
- More invasive change to the entire compiler
