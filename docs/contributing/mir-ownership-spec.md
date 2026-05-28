# MIR-3 Ownership Architecture Spec

## Instruction Contracts

Every `InstKind` declares what it expects from each operand and what it
produces. This is queryable via a `contract()` method.

### Operand conventions

```rust
enum OperandKind {
    /// Non-consuming read. Value must be live.
    Read,
    /// Consuming. Value must be @owned and live.
    Consume,
    /// Freezing borrow. Value must be @owned and live. Source becomes
    /// frozen (not consumable) until the corresponding EndBorrow.
    Freeze,
    /// Stack address operand.
    Addr,
}
```

### Result conventions

```rust
enum ResultKind {
    Owned,
    Guaranteed,
    Addr,
    Void,
    MultiOwned,
}
```

### Contract table

| Instruction | Operands | Result |
|---|---|---|
| `CopyValue { operand }` | `[Read]` | `Owned` |
| `MoveValue { operand }` | `[Consume]` | `Owned` |
| `DestroyValue { operand }` | `[Consume]` | `Void` |
| `BeginBorrow { operand }` | `[Freeze]` | `Guaranteed` |
| `EndBorrow { operand }` | `[Read]` | `Void` |
| `BeginMutBorrow { operand }` | `[Freeze]` | `Guaranteed` |
| `EndMutBorrow { operand }` | `[Read]` | `Void` |
| `Uninit { ty }` | `[]` | `Addr` |
| `FieldAddr { base, field }` | `[Addr]` | `Addr` |
| `Load { address }` | `[Addr]` | `Owned` |
| `CopyAddr { address }` | `[Addr]` | `Owned` |
| `Take { address }` | `[Addr]` | `Owned` |
| `BeginBorrowAddr { address }` | `[Addr]` | `Guaranteed` |
| `BeginMutBorrowAddr { address }` | `[Addr]` | `Guaranteed` |
| `StoreInit { address, value }` | `[Addr, Consume]` | `Void` |
| `StoreAssign { address, value }` | `[Addr, Consume]` | `Void` |
| `DestroyAddr { address }` | `[Addr]` | `Void` |
| `Struct { fields }` | `[Consume]*` | `Owned` |
| `Tuple { elements }` | `[Consume]*` | `Owned` |
| `Enum { payload }` | `[Consume]*` | `Owned` |
| `Array { elements }` | `[Consume]*` | `Owned` |
| `StructExtract { operand, field }` | `[Read]` | `Guaranteed` |
| `TupleExtract { operand, index }` | `[Read]` | `Guaranteed` |
| `EnumPayload { operand, variant }` | `[Read]` | `Guaranteed` |
| `DestructureStruct { operand }` | `[Consume]` | `MultiOwned` |
| `DestructureTuple { operand }` | `[Consume]` | `MultiOwned` |
| `DestructureEnum { operand }` | `[Consume]` | `MultiOwned` |
| `Discriminant { operand }` | `[Read]` | `Owned` |
| `Op1 { arg }` | `[Read]` | `Owned` \* |
| `Op2 { lhs, rhs }` | `[Read, Read]` \*\* | `Owned` |
| `Op3 { a, b, c }` | `[Read, Read, Read]` | `Owned` |
| `Literal { value }` | `[]` | `Owned` |
| `GlobalRef { entity }` | `[]` | `Owned` |
| `Call { callee, args }` | per-arg `ParamConvention` | `Owned` or `Void` |
| `ApplyPartial { captures }` | `[Consume]*` | `Owned` |

\* `Op1` with `PtrRead`: result is `Guaranteed` instead. Consider promoting
`PtrRead` to its own `InstKind` to eliminate this exception.

\*\* `Op2` with `PtrWrite`: rhs is `Consume` instead of `Read`. Consider
promoting `PtrWrite` to its own `InstKind` to eliminate this exception.

### Verifier

The verifier uses `contract()` in a generic loop instead of a per-instruction
match. Address init-state transitions (`Take` sets uninit, `StoreInit` requires
uninit, etc.) layer on top as instruction-specific post-checks.

---

## Lowerer Value Types

Zero-cost newtypes around `ValueId`, used only in the lowerer's emit API. The
underlying MIR continues to use `ValueId`.

```rust
struct Val(ValueId);         // unknown ownership
struct OwnedVal(ValueId);    // known @owned
struct BorrowedVal(ValueId); // known @guaranteed
struct StackAddr(ValueId);   // stack address (Uninit/FieldAddr)
```

All implement `Into<ValueId>` for constructing MIR instructions.

### Bridge methods

Adapt what you *have* to what you *need*:

```rust
impl OssaBodyCtx {
    /// Need @owned. BorrowedVal → CopyValue + EndBorrow. OwnedVal → identity.
    fn take_owned(&mut self, val: Val) -> OwnedVal;

    /// Need @guaranteed immutable. OwnedVal → BeginBorrow. BorrowedVal → identity.
    fn as_borrowed(&mut self, val: Val) -> BorrowedVal;

    /// Need @guaranteed mutable. OwnedVal → BeginMutBorrow.
    fn as_mut_borrowed(&mut self, val: Val) -> BorrowedVal;
}
```

### Call emission

One method replaces `emit_call_returning` and `emit_call_void`:

```rust
fn emit_call(&mut self, callee: Callee, args: Vec<(Val, ParamConvention)>, result_ty: TyId)
    -> Option<OwnedVal>
{
    let call_args = args.into_iter().map(|(val, conv)| {
        let adapted = match conv {
            Consuming => self.take_owned(val),
            Borrow => self.as_borrowed(val),
            MutBorrow => self.as_mut_borrowed(val),
        };
        CallArg { value: adapted.into(), convention: conv }
    }).collect();
    // emit Call, end borrows
}
```

---

## Scope Tracking

### Unified `ScopeEntry`

```rust
enum ScopeEntry {
    Owned(OwnedVal),                       // cleanup: DestroyValue
    Var { addr: StackAddr, ty: TyId },     // cleanup: DestroyAddr
}

struct ScopeFrame {
    entries: Vec<ScopeEntry>,
}
```

`ScopeSnapshot` saves and restores `entries`. One list — impossible to
forget a category.

### `jump_to_merge`

All merge-block jumps (if-then, if-else, no-else, match arms) go through one
method that handles `destroy_scope_except` + tracker values + jump. Can't
forget scope cleanup.

### Destroy instructions route through `push_inst`

`destroy_scope_except` and `destroy_scopes_to_depth` emit via `push_inst`, not
by pushing directly to `block.insts`. This gives them the `is_terminated()`
guard and span attachment.

### No deferred borrows

PtrRead scopes its borrow locally: `Op1(PtrRead) → CopyValue → EndBorrow`.
The `deferred_end_borrows` field and `drain_deferred_borrows()` mechanism are
removed. PtrRead borrows work like any other borrow — scoped at the creation
site, not drained at arbitrary call boundaries.

---

## Local Bindings

```rust
enum LocalBinding {
    /// Immutable let or borrow param. Used directly as SSA.
    Ssa(Val),
    /// Mutable var or mutating param. Load/Store through address.
    Var(StackAddr),
}

local_map: HashMap<HirLocalId, LocalBinding>
```

Replaces `local_map: HashMap<HirLocalId, ValueId>` +
`var_locals: HashSet<HirLocalId>`.

---

## Function Body Context

```rust
enum BodyContext {
    Normal,
    ProtocolExtension,
    Initializer { self_addr: StackAddr },
    ProtocolExtensionInit { self_addr: StackAddr },
}
```

Replaces `in_protocol_extension: bool` + `init_self_addr: Option<ValueId>`.
Field store dispatch checks `body_context` to choose `StoreInit` vs
`StoreAssign`.

---

## Convention Unification

### `ReceiverConvention` is `ParamConvention`

`FunctionKind::Method` does not carry a separate `ReceiverConvention`.
The receiver convention lives on `params[0].convention`.

### Helpers

```rust
fn receiver_convention(kind: &ReceiverKind) -> ParamConvention;
fn param_convention(param: &AstParam, is_extern: bool) -> ParamConvention;

fn conventions_from_callable(callable: &Callable, is_extern: bool) -> Vec<ParamConvention>;
```

`collect_conventions` checks MIR `FunctionDef` first, falls back to
`conventions_from_callable`. `collect_witness_conventions` resolves the entity
then calls `conventions_from_callable`.
