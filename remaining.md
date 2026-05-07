# Remaining Work — Optional/Throwing Constructors (#28)

## Completed

All original phases and deferred items are implemented:

- Parser + CST (`init()?`, `init() throws E`)
- AST Builder (`InitEffect` component + `TypeAnnotation`)
- HIR success wrapping (bare return + implicit tail)
- Type inference call-site wrapping (concrete + generic `T(args)`)
- MIR effectful init call handling (discriminant switch at call site)
- Definite init analysis (E009 relaxation for failure returns)
- Protocol conformance widening (non-failable satisfies failable)
- Conformance effect checking (E464)
- CLI abort on analyzer errors
- Partial-drop on init failure (field flags, failure return tagging, expand_deinit pass)

## Remaining

### 1. Stdlib init conformance fixes

Adding `NodeKind::Initializer` to the conformance completeness check exposes 5 stdlib types with init signature mismatches against their literal protocols:

- `Array` — `ExpressibleByArrayLiteral`
- `Dictionary` — `ExpressibleByDictionaryLiteral`
- `Set` — `ExpressibleByArrayLiteral`
- `String` — `ExpressibleByStringLiteral`
- `DefaultStringInterpolation` — `StringInterpolationProtocol`

The init labels or return types don't match the protocol requirements. These need to be fixed in the stdlib `.ks` files.

### 2. `try` with generic failable init

`try T(from: source)` inside a function returning `T?` gives "expected Optional[T] got T". The `try` operator correctly unwraps `T?` to `T`, but `T` doesn't coerce back to `Optional[T]` for generic type parameters. This is a generic coercion issue, not specific to init effects.

### 3. General deinit expansion

The `expand_deinit` pass is scoped to init-failure paths only. Widening it to all functions crashes because the deinit pass inserts `Deinit` for temporary locals that are copies of refcounted values (e.g., `String` temps) without the copy constructor incrementing the refcount. Expanding those into real deinit calls double-decrements the refcount and causes use-after-free.

The fix requires the deinit pass to perform ownership analysis — distinguishing locals that own their values from those that are borrowed or shared. This is a separate project from init effects.

### 4. Recursive field deinit

Types without an explicit `deinit` but with non-trivially-destructible fields (e.g., a struct containing a `String` but no custom `deinit`) don't get their fields cleaned up. The expand_deinit pass should recursively emit field-level deinits for these types. Depends on #3.

---

## Deinit Bugs Found by Fuzzing

Reproducers in `temp/fuzz/deinit/`. All bugs in `lib/kestrel-mir/src/passes/deinit.rs` unless noted.

### Bug A: Consuming function params never deinited (resource leak)

The pass filters all params with `id.index() >= body.param_count` (line 88), skipping consuming params. A consuming param transfers ownership to the callee, but no deinit is inserted. MIR for `func consume(consuming r: Resource) {}` shows just `return ()`.

Tests: 001, 003, 006, 007, 009, 012, 020, 023, 025, 026, 027, 030

### Bug B: Function call argument moves not tracked (double-free)

`find_moved_locals()` only scans `Assign` statements (`Rvalue::Move`, `Rvalue::Construct`). It does NOT detect moves in function call arguments (`call Foo(move %local)`). Locals passed by move get unconditional `Deinit` at scope exit instead of `DeinitIf`.

Tests: 001, 003, 009, 012, 023, 026, 027, 030

### Bug C: Reassignment of `var` does not deinit old value (resource leak)

When a `var` of a non-copyable type is reassigned, the old value must be deinited before the new value is stored. The pass only inserts deinits at Return terminators, not at assignment statements.

```kestrel
var x = Resource(id: 1);
x = Resource(id: 2);  // Resource(id: 1) is LEAKED
```

Tests: 004, 005, 006, 022

### Bug D: Labeled `break` misses intermediate scope deinits + double-deinits

`break outer` from nested loops only deinits the innermost loop's variable. Variables from intermediate loops are skipped. The break-time deinit is not tracked by any flag, so the scope-exit deinit fires again (double-free).

Tests: 015

### Bug E: Return value deinited before return (use-after-deinit)

When a function constructs a value and returns it, the pass inserts an unconditional Deinit before the Return — deiniting the value that's about to be returned. Test 014 "passes" only because this accidentally increments the counter to the expected value.

Tests: 014 (false pass)

### Bug F: Guard-let + Optional produces garbage deinits

`guard let` with Optional unwrapping produces phantom deinits of `id=0` resources (reading uninitialized memory) and double-deinits. 9 deinits fire when only 2 are expected.

Tests: 017

### Compiler stack overflow (test 028)

`indirect enum Tree` with recursive non-copyable payloads causes stack overflow during compilation — infinite recursion in expand_deinit.

### Fix priority

1. Bug B — double-free (crash risk)
2. Bug A — resource leaks
3. Bug E — use-after-deinit
4. Bug C — resource leaks
5. Bug D — double-free + leak in loops
6. Bug F — may cascade from B+D
