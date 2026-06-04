# Lowering to OSSA

How the HIR→OSSA lowerer (`kestrel-mir-lower`) emits ownership-correct IR.
The lowerer must emit copy_value, destroy_value, begin_borrow/end_borrow,
and block arguments correctly so the resulting body verifies under OSSA.

## Architecture

The lowerer (`kestrel-mir-lower/src/body_lower.rs`) is built around a few
focused choke points:

1. **Single statement emission choke point**: `emit_stmt()`
2. **Single value-mode decision point**: `emit_value_transfer()`
3. **Modular control flow handlers**: `lower_if`, `lower_loop`, `lower_match`
   are independent, self-contained functions
4. **Scope tracking**: the scope-frame stack and its live-value markers

The design rests on a handful of invariants:
- Values are `ValueId`s, not addressable locals.
- `emit_value_transfer` emits `CopyValue`/`MoveValue` to model ownership transfer.
- Control flow handlers thread complete owned live-in sets through block
  arguments rather than through shared mutable locals.
- Scope exits emit `DestroyValue` for unconsumed @owned values.
- Place reads/writes use explicit address ownership operations:
  `Load` for trivial values, `CopyAddr`, `Take`, `BeginBorrowAddr`,
  `StoreInit`, `StoreAssign`, and `DestroyAddr` for non-trivial memory.

## Scope Tracking

The lowerer maintains a stack of scope frames that track which @owned
values are live and unconsumed:

```rust
struct ScopeFrame {
    /// @owned values created in or inherited by this scope that haven't
    /// been consumed yet. Removed when consumed (moved, returned, etc.).
    owned_values: Vec<ValueId>,
}

struct OssaLowerCtx {
    /// Stack of scope frames. Innermost scope is last.
    scopes: Vec<ScopeFrame>,
    // ...
}
```

### Operations on Scopes

```rust
impl OssaLowerCtx {
    /// Register a new @owned value in the current scope.
    fn track_owned(&mut self, value: ValueId) {
        self.scopes.last_mut().unwrap().owned_values.push(value);
    }

    /// Mark a value as consumed (moved, forwarded, returned).
    /// Removes it from whatever scope tracks it.
    fn consume(&mut self, value: ValueId) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(pos) = scope.owned_values.iter().position(|&v| v == value) {
                scope.owned_values.remove(pos);
                return;
            }
        }
        // value was @none. If it was @owned and already consumed, the
        // verifier should catch the double-consume in the emitted IR.
    }

    /// Emit destroy_value for all unconsumed @owned values in the
    /// current scope. Called before scope exit (return, break, continue,
    /// or fall-through to a block that doesn't accept them).
    fn destroy_scope(&mut self) {
        let scope = self.scopes.last().unwrap();
        for &value in scope.owned_values.iter().rev() {
            self.emit(DestroyValue { operand: value });
        }
    }

    /// Emit destroy_value for all unconsumed @owned values in the
    /// current scope EXCEPT the specified values. Removes destroyed
    /// values from the scope's tracking list. The kept values remain
    /// tracked. Used before jumps that thread specific values as
    /// block arguments.
    fn destroy_scope_except(&mut self, keep: &[ValueId]) {
        let scope = self.scopes.last_mut().unwrap();
        let mut surviving = Vec::new();
        for &value in scope.owned_values.iter().rev() {
            if keep.contains(&value) {
                surviving.push(value);
            } else {
                self.emit(DestroyValue { operand: value });
            }
        }
        surviving.reverse();
        scope.owned_values = surviving;
    }

    /// Destroy all unconsumed @owned values across ALL scopes from
    /// the current scope up to (but not including) the specified
    /// scope depth. Used by break/continue which exit multiple
    /// nested scopes at once (e.g., break inside an if inside a loop).
    fn destroy_scopes_to_depth(&mut self, target_depth: usize, keep: &[ValueId]) {
        for scope in self.scopes[target_depth..].iter().rev() {
            for &value in scope.owned_values.iter().rev() {
                if !keep.contains(&value) {
                    self.emit(DestroyValue { operand: value });
                }
            }
        }
    }

    /// Pop the current scope and emit destroys for anything left.
    fn exit_scope(&mut self) {
        self.destroy_scope();
        self.scopes.pop();
    }

    /// Pop the current scope WITHOUT emitting destroys. Used after
    /// destroy_scope_except has already destroyed what's needed and
    /// the remaining values have been threaded as block arguments.
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Return all @owned values across the entire scope stack.
    /// Used by loop lowering to determine what values must be
    /// threaded through the header block.
    fn all_live_owned(&self) -> Vec<(ValueId, TyId)> {
        self.scopes.iter()
            .flat_map(|s| &s.owned_values)
            .map(|&v| (v, self.value_type(v)))
            .collect()
    }
}
```

## Copy vs Move Decision

The lowerer decides at emit time whether to copy or move a value:

```
Source type is Bitwise (Int64, Bool, etc.):
  → Use the ValueId directly. @none, no copy needed.

Source type is Clone:
  if source will be used again in this scope:
    → Emit CopyValue. Produces new @owned. Track it.
    → Original stays alive in scope.
  else (last use):
    → Consume the original directly.
    → No CopyValue needed (implicit move).

Source type is Affine (not copyable):
  → Must be last use. Consume directly.
  → If used again, this is a compile error caught by type-checker
    before MIR lowering.
```

### "Will be used again" detection

The lowerer doesn't need liveness analysis for this. Instead:

1. **For let bindings**: The HIR records every reference to a local. The
   lowerer can check if there are remaining uses after the current point.
   In practice, the lowerer can use a simple reference count per HIR
   local, decremented at each use.

2. **For temporaries**: Temps are single-use by construction. Always move.

3. **Conservative default**: If unsure, emit CopyValue. The copy_optimize
   pass will eliminate unnecessary copies later. Correctness does not
   depend on optimal copy/move decisions — the verifier checks that
   every value is consumed exactly once regardless.

### emit_value_transfer

`emit_value_transfer` in `body_lower.rs` is the single canonicalized site
where the copy-vs-move decision is resolved into an SSA value:

```rust
fn emit_value_transfer(&mut self, value: ValueId, for_type: TyId) -> ValueId {
    let ownership = ownership_for_type(for_type, self.module);
    match ownership {
        Ownership::None => {
            // Trivial — use directly, no copy/move
            value
        }
        Ownership::Owned => {
            if self.is_last_use(value) {
                // Move: consume the value
                self.consume(value);
                value
            } else {
                // Copy: keep original, produce new owned
                let copy = self.new_value(for_type, Ownership::Owned);
                self.emit(CopyValue { result: copy, operand: value });
                self.track_owned(copy);
                copy
            }
        }
        Ownership::Guaranteed => {
            // Should not happen in value transfer — guaranteed values
            // are created by begin_borrow, not transfers
            unreachable!()
        }
    }
}
```

## Control Flow Lowering

### If/Else

The result of an `if` expression flows through block arguments: each arm
jumps to a merge block, passing its result as a block argument.

```rust
fn lower_if(&mut self, cond: ValueId, then_body: &HirBlock, else_body: &HirBlock) -> ValueId {
    let result_ty = ...;
    let ownership = ownership_for_type(result_ty, self.module);

    let then_bb = self.new_block();
    let else_bb = self.new_block();
    let merge_bb = self.new_block_with_param(result_ty, ownership);

    self.emit_terminator(Branch {
        condition: cond,
        then_block: then_bb, then_args: vec![],
        else_block: else_bb, else_args: vec![],
    });

    // Then arm
    self.switch_to(then_bb);
    self.push_scope();
    let then_val = self.lower_block(then_body);
    self.destroy_scope_except(&[then_val]);  // destroy everything except the result
    self.emit_terminator(Jump { target: merge_bb, args: vec![then_val] });
    self.pop_scope();

    // Else arm
    self.switch_to(else_bb);
    self.push_scope();
    let else_val = self.lower_block(else_body);
    self.destroy_scope_except(&[else_val]);
    self.emit_terminator(Jump { target: merge_bb, args: vec![else_val] });
    self.pop_scope();

    // Merge
    self.switch_to(merge_bb);
    let result = self.block_param(merge_bb, 0);
    if ownership == Ownership::Owned {
        self.track_owned(result);
    }
    result
}
```

The result flows through block arguments. In real lowering, the merge
block also receives any `@owned` values still live after the `if`
expression. Each arm either passes those values to the merge block or
consumes/destroys them before jumping. The block parameter list is the
complete owned live-in set, not just the expression result.

### Loops

Loops use block arguments to thread live values through the loop: the
header block takes the complete owned live-in set as parameters, and the
back-edge re-passes the current values.

```rust
fn lower_loop(&mut self, body: &HirBlock) -> ValueId {
    // Collect all @owned values live at loop entry (across all scopes)
    let live_owned: Vec<(ValueId, TyId)> = self.all_live_owned();

    // Create header block with params for the complete owned live-in set
    let header = self.new_block_with_params(&live_owned);
    let exit = self.new_block();

    // Jump to header, passing current values
    let initial_args: Vec<ValueId> = live_owned.iter().map(|(v, _)| *v).collect();
    self.emit_terminator(Jump { target: header, args: initial_args });

    // In header: replace scope values with block params
    self.switch_to(header);
    let header_params: Vec<ValueId> = (0..live_owned.len())
        .map(|i| self.block_param(header, i))
        .collect();
    self.rebind_scope_values(&initial_args, &header_params);

    // Push loop context for break/continue
    self.push_loop(header, exit, &header_params);

    // Lower body
    self.push_scope();
    self.lower_block(body);

    // Back-edge: jump to header with current values
    if !self.is_terminated() {
        let current_vals = self.current_owned_matching(&header_params);
        self.emit_terminator(Jump { target: header, args: current_vals });
    }
    self.pop_scope();

    self.pop_loop();
    self.switch_to(exit);
    Value::unit()
}
```

### Break/Continue

A break or continue may exit multiple nested scopes (e.g., break inside
an if inside a loop). All scopes from the current scope up to the loop's
scope must be destroyed.

```rust
fn lower_break(&mut self) {
    let loop_info = self.current_loop();
    let exit = loop_info.exit;
    let loop_depth = loop_info.scope_depth;
    // Destroy all scopes between here and the loop exit,
    // keeping nothing (break exits entirely)
    self.destroy_scopes_to_depth(loop_depth, &[]);
    self.emit_terminator(Jump { target: exit, args: vec![] });
}

fn lower_continue(&mut self) {
    let loop_info = self.current_loop();
    let header = loop_info.header;
    let loop_depth = loop_info.scope_depth;
    let expected_params = loop_info.header_params.clone();
    // Destroy all intermediate scopes, keeping values that
    // must be threaded back to the loop header
    self.destroy_scopes_to_depth(loop_depth, &expected_params);
    let current_vals = self.current_owned_matching(&expected_params);
    self.emit_terminator(Jump { target: header, args: current_vals });
}
```

### Match

Match is similar to if/else but with N arms. Each arm's result flows
to the merge block via block arguments, along with any other `@owned`
values live after the match:

```rust
fn lower_match(&mut self, scrutinee: ValueId, arms: &[HirMatchArm]) -> ValueId {
    let result_ty = ...;
    let merge_bb = self.new_block_with_param(result_ty, ownership_for_type(result_ty));

    // Emit switch on scrutinee
    // For each arm: lower body, destroy scope, jump merge(result, live_owned...)
    // Scrutinee is consumed by the switch (for @owned enums)
    // or read (for @none discriminants like Bool/Int)

    self.switch_to(merge_bb);
    self.block_param(merge_bb, 0)
}
```

### Try/Error

Try expressions are desugared before MIR lowering (in HIR). The lowerer
sees the desugared form (match on Result enum). The error path destroys
scope locals and returns the error value. The success path extracts the
Ok payload and continues.

## Call Argument Lowering

For each call argument:

```rust
fn lower_call_arg(&mut self, value: ValueId, convention: ParamConvention) -> CallArg {
    match convention {
        ParamConvention::Consuming => {
            // Transfer ownership to callee
            self.consume(value);
            CallArg { value, convention }
        }
        ParamConvention::Borrow => {
            // Create ephemeral borrow of an owned SSA value
            let borrow = self.new_value(value_ty, Ownership::Guaranteed);
            self.emit(BeginBorrow { result: borrow, operand: value });
            // end_borrow emitted after the call
            CallArg { value: borrow, convention }
        }
        ParamConvention::MutBorrow => {
            let borrow = self.new_value(value_ty, Ownership::Guaranteed);
            self.emit(BeginMutBorrow { result: borrow, operand: value });
            CallArg { value: borrow, convention }
        }
    }
}
```

For address-backed places, the borrowed cases use address borrows:

```rust
fn lower_call_place_arg(&mut self, address: ValueId, convention: ParamConvention) -> CallArg {
    match convention {
        ParamConvention::Borrow => {
            let borrow = self.new_value(value_ty, Ownership::Guaranteed);
            self.emit(BeginBorrowAddr { result: borrow, address, ty: value_ty });
            CallArg { value: borrow, convention }
        }
        ParamConvention::MutBorrow => {
            let borrow = self.new_value(value_ty, Ownership::Guaranteed);
            self.emit(BeginMutBorrowAddr { result: borrow, address, ty: value_ty });
            CallArg { value: borrow, convention }
        }
        ParamConvention::Consuming => {
            let taken = self.new_value(value_ty, Ownership::Owned);
            self.emit(Take { result: taken, address, ty: value_ty });
            CallArg { value: taken, convention }
        }
    }
}
```

After the call, the lowerer emits `EndBorrow`/`EndMutBorrow` for all
borrowed arguments. This keeps borrow scopes tight and local.

## Place Reads and Writes

When lowering a place expression, the lowerer must choose the ownership
operation explicitly:

```rust
fn read_place(&mut self, address: ValueId, ty: TyId, intent: ReadIntent) -> ValueId {
    match (ownership_for_type(ty), intent) {
        (Ownership::None, _) => self.emit_load(address, ty),
        (Ownership::Owned, ReadIntent::Copy) => self.emit_copy_addr(address, ty),
        (Ownership::Owned, ReadIntent::Move) => self.emit_take(address, ty),
        (Ownership::Owned, ReadIntent::Borrow) => self.emit_begin_borrow_addr(address, ty),
        (Ownership::Guaranteed, _) => unreachable!(),
    }
}

fn write_place(&mut self, address: ValueId, value: ValueId, initialized: bool) {
    if initialized {
        self.emit(StoreAssign { address, value });
    } else {
        self.emit(StoreInit { address, value });
    }
}
```

The lowerer is responsible for maintaining initialized/uninitialized
state for address-backed storage such as locals, fields, array elements,
globals, and `self` during initialization. The verifier checks the
emitted address effects: `take` and `destroy_addr` require initialized
memory and leave it uninitialized; `store_init` requires uninitialized
memory; `store_assign` requires initialized memory and destroys the old
contents before storing the new value.

## Initializer Lowering (Self-Init)

Initializers start with `self` uninitialized. The lowerer:

1. Emits `Uninit { ty }` to allocate stack space for `self`. Result is
   @none (a pointer to uninitialized memory).
2. For each field assignment `self.field = value`, emits `FieldAddr` to
   get the typed field address, then `StoreInit` to initialize it.
3. After all fields are initialized, the whole `self` is considered
   initialized. To return `self` as @owned, emit `Take` on the base
   address. This moves the value out of memory into an @owned SSA value.
4. Return the @owned value.

The verifier tracks per-field init state through `FieldAddr` projections
(see "Address State Tracking" in passes.md). Each field starts
uninitialized, transitions to initialized via `StoreInit`, and all
fields must be initialized before `Take` on the whole address is valid.

```
fn init(%x_val: @owned Int64, %s_val: @owned String):
    %self_addr = uninit MyStruct           // @none pointer, all fields uninit
    %x_addr = field_addr %self_addr, MyStruct, .x  // @none, typed field addr
    store_init %x_addr, %x_val             // field .x now initialized
    %s_addr = field_addr %self_addr, MyStruct, .s  // @none, typed field addr
    store_init %s_addr, %s_val             // field .s now initialized
    %self = take %self_addr, MyStruct      // @owned, all fields init'd
    return %self
```

For failable initializers, the failure path must `DestroyAddr` any
fields that were already initialized before returning the error:

```
fn try_init(%s_val: @owned String) throws:
    %self_addr = uninit MyStruct
    %s_addr = field_addr %self_addr, MyStruct, .s
    store_init %s_addr, %s_val             // .s initialized
    %result = call validate()              // may fail
    // ... on error path:
    destroy_addr %s_addr, String           // clean up .s (→ uninit)
    // %self_addr is never Take'd — all fields back to uninit
```

## Lowerer Building Blocks

The lowerer (`body_lower.rs`) is organized around these pieces:

| Component | Role |
|-----------|------|
| `emit_stmt()` | Statement emission choke point |
| `emit()` | Appends an OSSA instruction to the current block |
| `emit_value_transfer()` | Copy vs Move decision point |
| `arg_for_value()` | Convention-based borrow decision |
| `apply_callee_param_modes()` | Emits begin_borrow/end_borrow around calls |
| Scope frame stack | Tracks live @owned values per scope |
| Block arg threading | Carries live values through loop headers |
| `new_value()` | Allocates a fresh `ValueId` |
| `switch_to_block()` | Selects the active block |
| `set_terminator()` | Sets a block's terminator, with block args |

These cooperate to satisfy OSSA's core obligations:
1. Scope frame stack tracking live @owned values
2. Block argument threading in control flow handlers
3. Borrow insertion around calls
4. Destroy emission at scope exits
