# Calling Conventions

How Kestrel function signatures map to MIR parameters, call-site modes,
and ABI decisions.

## User-facing model

Kestrel has three parameter conventions, specified by keyword:

| Keyword    | Meaning                              | Default? |
|-----------|--------------------------------------|----------|
| (none)    | borrow — immutable reference         | yes      |
| `mutating`| mutable reference                    | no       |
| `consuming`| ownership transfer                  | no       |

Borrow is the default. Most parameters are passed by reference without
the user writing any keyword.

## MIR representation

### ParamDef

```rust
struct ParamDef {
    local: LocalId,
    ty: TyId,                        // the unwrapped type (e.g. String, not Pointer(String))
    convention: ParamConvention,
}

enum ParamConvention {
    Borrow,      // immutable reference — callee sees Pointer(T)
    MutBorrow,   // mutable reference — callee sees Pointer(T)
    Consuming,   // ownership transfer — callee sees T directly
}
```

The parameter's type in ParamDef is always the *semantic* type — `String`,
not `Pointer(String)`. The convention tells codegen how to pass it.

### ParamDef.ty vs LocalDef.ty

ParamDef and LocalDef store DIFFERENT types for borrow params:

| Convention | ParamDef.ty | LocalDef.ty | Why |
|-----------|------------|------------|-----|
| Borrow    | String     | Pointer(String) | Local physically holds a pointer |
| MutBorrow | String     | Pointer(String) | Local physically holds a pointer |
| Consuming | String     | String          | Local holds the value directly |

The lowering sets `LocalDef.ty` based on the convention: Borrow/MutBorrow
wraps the semantic type in `Pointer(...)`. The verifier checks consistency:
for a param with `convention != Consuming`, `LocalDef.ty` must equal
`Pointer(ParamDef.ty)`. For Consuming, they must be equal.

Inside the function body, reads from a borrow param go through the pointer
(codegen emits a load). Reads from a consuming param access the value
directly.

### Call-site encoding

At call sites, arguments carry ArgMode:

```rust
Call {
    dest: Option<Place>,
    callee: Callee,
    args: Vec<(Operand, ArgMode)>,
}

enum ArgMode {
    Copy,    // pass by value, source retained
    Move,    // pass by value, source consumed
    Ref,     // create ephemeral &, pass pointer
    RefMut,  // create ephemeral &var, pass pointer
}
```

The lowering maps convention + type to ArgMode:

| Callee convention | Type's CopyBehavior | ArgMode |
|-------------------|---------------------|---------|
| Borrow            | (any)               | Ref     |
| MutBorrow         | (any)               | RefMut  |
| Consuming         | Bitwise             | Copy    |
| Consuming         | Clone               | Move *  |
| Consuming         | None (affine)       | Move    |

\* For consuming Clone args, the **lowering** (not clone elaboration) is
responsible for inserting the clone. The lowering emits
`_clone = call Cloneable.clone(ref x); call foo(move _clone)`.
Clone elaboration does not rewrite call args — it only handles
assignments, composite rvalue fields, and returns.

### Why ArgMode::Ref instead of materializing a temp

Borrow is the default convention. Materializing every borrow arg would mean:

```
_ref = ref array              // extra temp
call array.count(move _ref)   // extra statement
```

For a language where nearly every method call borrows self, this doubles
the statement count for method-heavy code. ArgMode::Ref avoids this by
expressing "create an ephemeral reference for this call" inline.

The ephemeral reference has a trivially bounded lifetime (the call).
A future borrow checker can handle call-scoped borrows specially
because they're structurally distinct (ArgMode::Ref) from standalone
reference creation (Rvalue::Ref).

## Receiver conventions

Methods have an implicit self parameter. The receiver convention maps to
ParamConvention:

| Kestrel syntax         | ParamConvention |
|------------------------|-----------------|
| `func foo()`           | Borrow          |
| `mutating func foo()`  | MutBorrow       |
| `consuming func foo()` | Consuming       |

The receiver is always `params[0]`. The FunctionDef carries the receiver
convention as part of FunctionKind::Method for passes that need to know
without inspecting params.

## Closure captures

Closures capture variables from the enclosing scope. Captures can be:

- **By value** (Copy or Move): the capture is an owned copy of the value.
  The closure's env struct holds the value directly.
- **By reference**: the capture borrows the value from the parent scope.
  The closure's env struct holds a Pointer(T).

In the MIR, borrowed captures are materialized as ref temps before the
ApplyPartial rvalue:

```
_ref_x = ref x                                        // Rvalue::Ref(x)
_closure = apply_partial f, captures: [move _ref_x]   // UseMode::Move
```

This keeps ApplyPartial captures on UseMode (Copy|Move) rather than
introducing ArgMode in compound rvalue position. Closures are rare (~1-2
per function), so the extra temp is acceptable.

**The `borrowed` flag:** Inside the closure's body (not the parent), the
parameter local that receives the captured reference has `borrowed: true`
on its LocalDef. This tells drop elaboration: "this local is a borrowed
view of someone else's value — don't drop it." The flag goes on the
closure's param local, NOT on the original variable in the parent scope.
The original variable in the parent scope is still owned and will be
dropped normally by the parent function's drop elaboration.

## ABI mapping

Codegen maps ParamConvention to ABI:

| ParamConvention | ABI                              |
|-----------------|----------------------------------|
| Borrow          | pass as const pointer            |
| MutBorrow       | pass as mutable pointer          |
| Consuming       | pass by value (or by pointer for aggregates, per platform ABI) |

For aggregate types passed by value, the platform ABI may require passing
a pointer to a caller-allocated copy. This is a codegen concern — the MIR
always represents Consuming params as bare T.
