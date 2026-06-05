# Codegen

How the Cranelift codegen backend consumes OSSA.

## Overview

The codegen backend (`kestrel-codegen-cranelift`) matches on the MIR's
`InstKind`, `Value`, and `TerminatorKind` across the instruction- and
terminator-compilation functions. Monomorphization has already happened
as a MIR pass, so every type the backend sees is concrete: codegen never
substitutes type parameters, it only reads concrete layouts and emits
native code. Type layout and symbol mangling live in `kestrel-mir`; the
backend-agnostic `kestrel-codegen` crate provides only target
configuration (`TargetConfig`). Cranelift natively supports every IR
feature we rely on — especially block arguments.

## Block Arguments

Values flow between blocks through Cranelift's native block params, which
mirror OSSA's block arguments one-to-one:

```rust
// Declare block with params
let bb3 = builder.create_block();
builder.append_block_param(bb3, types::I64);      // for @none
builder.append_block_param(bb3, types::I64);      // pointer for @owned aggregate

// Jump with args
builder.ins().jump(bb3, &[value1, value2]);

// Read params in target block
builder.switch_to_block(bb3);
let param0 = builder.block_params(bb3)[0];
let param1 = builder.block_params(bb3)[1];
```

Cranelift handles phi insertion natively, so ownership-relevant values
flow directly through block params without any `Variable` indirection.

The `Variable` system is reserved for values that are modified within a
block (e.g., accumulator patterns):

```rust
let var = Variable::new(local.index());
builder.declare_var(var, cranelift_type);
builder.def_var(var, value);    // at definition
let val = builder.use_var(var); // at use
```

`Variable` is Cranelift's SSA builder — it inserts phi nodes
automatically. It is needed only for in-place mutation; every other
value uses block params.

## Instruction Compilation

### Value Lifecycle

```rust
CopyValue { result, operand } => {
    // For Clone types: emit a clone() witness call.
    // clone() takes self by Borrow (pointer).
    let ty = value_type(operand);
    let clone_func = resolve_witness("Cloneable", "clone", ty);
    let ref_val = if ownership_of(operand) == Guaranteed {
        // @guaranteed values are already addresses (from BeginBorrow).
        // Pass directly — no need to take_address again.
        get_value(operand)
    } else {
        // @owned values need their stack address taken.
        take_address(operand)
    };
    let result_val = emit_call(clone_func, &[ref_val]);
    map_value(result, result_val);
}

MoveValue { result, operand } => {
    // Pure rename — just alias the Cranelift value
    let val = get_value(operand);
    map_value(result, val);
}

DestroyValue { operand } => {
    let ty = value_type(operand);
    if needs_drop(ty) {
        // Call drop shim
        let shim = find_drop_shim(ty);
        let val = get_value(operand);
        emit_call(shim, &[val]);
    }
    // For non-droppable types: no-op (value just dies)
}
```

### Borrowing

```rust
BeginBorrow { result, operand } => {
    // Take address of the owned value
    let addr = take_address(operand);
    map_value(result, addr);
}

EndBorrow { operand } => {
    // No-op at codegen level — borrow scope is a compile-time concept
}

BeginMutBorrow { result, operand } => {
    let addr = take_address(operand);
    map_value(result, addr);
}

EndMutBorrow { operand } => {
    // No-op
}

BeginBorrowAddr { result, address, .. } => {
    map_value(result, get_value(address));
}

BeginMutBorrowAddr { result, address, .. } => {
    map_value(result, get_value(address));
}
```

Borrows compile to pointer operations. The borrow scope enforcement is
a compile-time property checked by the verifier, not a runtime cost.

### Memory Access

```rust
Load { result, address } => {
    let addr = get_value(address);
    // Only valid for @none/trivial result types.
    let loaded = compile_load(addr, value_type(result));
    map_value(result, loaded);
}

CopyAddr { result, address, ty } => {
    let addr = get_value(address);
    // Clone/copy from initialized memory without consuming it.
    let clone_func = resolve_witness("Cloneable", "clone", ty);
    let copied = emit_call(clone_func, &[addr]);
    map_value(result, copied);
}

Take { result, address, ty } => {
    let addr = get_value(address);
    // Move out of memory. For aggregates this usually means returning the
    // address/pointer representation; the verifier now treats addr as uninit.
    let moved = compile_load_or_addr(addr, ty);
    map_value(result, moved);
}

StoreInit { address, value } => {
    let addr = get_value(address);
    let val = get_value(value);
    compile_store(addr, val, value_type(value));
}

StoreAssign { address, value } => {
    let addr = get_value(address);
    let old_ty = pointee_type(address);
    if needs_drop(old_ty) {
        emit_drop_addr(addr, old_ty);
    }
    let val = get_value(value);
    compile_store(addr, val, value_type(value));
}

DestroyAddr { address, ty } => {
    let addr = get_value(address);
    if needs_drop(ty) {
        emit_drop_addr(addr, ty);
    }
}
```

The address-state instructions (`CopyAddr`, `Take`, `StoreInit`,
`StoreAssign`, `DestroyAddr`) make memory ownership explicit. Codegen
emits the corresponding load/copy/store/drop-address sequence directly,
rather than treating place reads and writes uniformly.

### Computation

```rust
Op1/Op2/Op3 => {
    // Read operand values, dispatch op
    let args = get_values(&[arg/lhs/rhs/a/b/c]);
    let result_val = dispatch_op(op, args);
    map_value(result, result_val);
}
```

### Address Projection

```rust
FieldAddr { result, base, ty, field } => {
    let base_val = get_value(base);
    let offset = field_offset(ty, field);
    let addr = builder.ins().iadd_imm(base_val, offset as i64);
    map_value(result, addr);
}
```

### Discriminant

```rust
Discriminant { result, operand } => {
    // Read the integer tag from an enum's memory representation
    // without consuming the enum. The tag is at offset 0 (I32).
    let base = get_value(operand);
    let tag = builder.ins().load(types::I32, MemFlags::trusted(), base, 0);
    map_value(result, tag);
}
```

### Constants

```rust
Literal { result, value } => {
    let val = compile_immediate(value);
    map_value(result, val);
}

GlobalRef { result, entity } => {
    let addr = resolve_global(entity);
    map_value(result, addr);
}
```

### Aggregates

Construction and destructuring compile to pointer/offset arithmetic. The
ownership information is compile-time only — at the codegen level, struct
construction is "write fields to offsets" and extraction is "read from
offset."

```rust
Struct { result, ty, fields } => {
    let ptr = stack_alloc(size_of(ty));
    for (idx, value) in fields {
        let offset = field_offset(ty, idx);
        store_at(ptr, offset, get_value(value));
    }
    map_value(result, ptr);
}

StructExtract { result, operand, field } => {
    let base = get_value(operand);
    let offset = field_offset(operand_ty, field);
    let val = load_from(base, offset);
    map_value(result, val);
}
```

### Calls

```rust
Call { result, callee, args } => {
    // Convention on CallArg determines ABI (by-value vs by-ref)
    let compiled_args = args.iter().map(|arg| {
        match arg.convention {
            Consuming => get_value(arg.value),  // pass by value
            Borrow | MutBorrow => get_value(arg.value), // already a pointer from begin_borrow
        }
    });
    let result_val = emit_call(callee, compiled_args);
    if let Some(r) = result {
        map_value(r, result_val);
    }
}
```

Because monomorphization runs as a MIR pass, every `callee` is a direct
reference to a concrete, fully-substituted function — codegen resolves it
to a symbol and emits a direct call.

### Terminators

```rust
Return(value) => {
    let val = get_value(value);
    // sret, scalar, or aggregate return per the calling convention
    compile_return(val);
}

Jump { target, args } => {
    let cl_block = block_map[target];
    let cl_args: Vec<_> = args.iter().map(|v| get_value(v)).collect();
    builder.ins().jump(cl_block, &cl_args);
}

Branch { condition, then_block, then_args, else_block, else_args } => {
    let cond = get_value(condition);
    let then_cl = block_map[then_block];
    let else_cl = block_map[else_block];
    let then_vals: Vec<_> = then_args.iter().map(|v| get_value(v)).collect();
    let else_vals: Vec<_> = else_args.iter().map(|v| get_value(v)).collect();
    builder.ins().brif(cond, then_cl, &then_vals, else_cl, &else_vals);
}

Switch { discriminant, cases } => {
    // Each case target receives block arguments
}
```

## Ownership in Codegen

Ownership is decided in the IR, not in codegen. The backend never
chooses between copy and move per operand — `CopyValue`, `MoveValue`,
and the address-state instructions have already encoded that decision.
Codegen just reads values and emits native code:

- **Block argument threading.** Every jump/branch/switch compiles its
  argument list, threading ownership-relevant values through native
  block params.

- **`CopyValue` → clone call.** Resolves the Cloneable witness and emits
  a call. A pre-codegen "copy lowering" pass can turn `CopyValue` into an
  explicit `Call` instruction so codegen stays simple (see below).

- **Value mapping.** Codegen maps `ValueId → Cranelift Value` with a flat
  lookup — no `def_var` / `use_var` dance for block-threaded values.

- **Explicit address state.** `CopyAddr`, `Take`, `StoreInit`,
  `StoreAssign`, and `DestroyAddr` make memory ownership explicit, so the
  backend emits the exact load/copy/store/drop-address sequence each one
  describes.

## Pre-Codegen Copy Lowering (Recommended)

To keep codegen simple, run a "copy lowering" pass before codegen that
rewrites `CopyValue` instructions into explicit `Call` instructions:

```
// Before (OSSA):
%copy = copy_value %original

// After (copy-lowered):
%ref = begin_borrow %original
%copy = call Cloneable.clone(%ref)    // Witness call
end_borrow %ref
```

This way, codegen never sees `CopyValue` — it only sees `Call`
instructions, which it already knows how to compile. The copy lowering
pass runs after `ossa_verify` (the verifier checks copy_value semantics)
and before codegen.

## File Layout

| File | Responsibility |
|-------------|---------|
| `block.rs` | `compile_statement` matches `InstKind` |
| `terminator.rs` | Jumps/branches/switches pass block args |
| `function.rs` | Block param setup, value mapping |
| `rvalue/call.rs` | Call compilation (convention → ABI) |
| `rvalue/construct.rs` | Aggregate construction/extraction |
