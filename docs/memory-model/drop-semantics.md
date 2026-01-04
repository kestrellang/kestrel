# Drop Semantics (RAII)

Kestrel uses RAII (Resource Acquisition Is Initialization) for deterministic resource cleanup through `deinit`.

## The `deinit` Block

Types can define cleanup logic that runs when a value goes out of scope:

```kestrel
struct FileHandle: not Copyable {
    var fd: Int
    
    deinit {
        close(self.fd)
    }
}

func example() {
    let f = FileHandle(fd: open("file.txt"))
    // use f...
}  // f.deinit called here, file closed
```

## Drop Order

Values are dropped in **reverse order of declaration**:

```kestrel
func example() {
    let a = Resource("a")
    let b = Resource("b")
    let c = Resource("c")
}  // Drops: c, b, a
```

This ensures resources that depend on earlier resources are cleaned up first.

## Drop Rules

### 1. Scope Exit

Values are dropped when they go out of scope:

```kestrel
func example() {
    let f = FileHandle(...)
    if condition {
        let g = FileHandle(...)
        // use g
    }  // g dropped here
    // f still valid
}  // f dropped here
```

### 2. Move Semantics

If a value is **moved**, its `deinit` is **NOT** called at the source. The new owner is responsible:

```kestrel
func consume(consuming f: FileHandle) {
    // f is owned here
}  // f.deinit called here

func example() {
    let f = FileHandle(...)
    consume(f)  // f is moved
}  // f.deinit NOT called here (already moved)
```

### 3. Copy Semantics

For Copyable types without custom `deinit`, copies are independent:

```kestrel
struct Point {
    var x: Int
    var y: Int
    // No deinit - trivially copyable
}

let p1 = Point(x: 1, y: 2)
let p2 = p1  // Copy
// Both p1 and p2 are valid and independent
```

### 4. Early Drop with `drop()`

Values can be explicitly dropped before scope end using the `drop()` intrinsic:

```kestrel
func example() {
    var f = FileHandle(...)
    // ... use f ...
    drop(f)    // Explicitly drop f now, calls deinit
    // f is invalid after this point
    
    print("file is closed")
}
```

The `drop()` intrinsic:
- Immediately runs the value's `deinit` (if any)
- Marks the variable as moved/invalid
- Is useful for releasing resources before scope end

## Struct Field Drop Order

When a struct is dropped, fields are dropped in **reverse declaration order**:

```kestrel
struct Container {
    var first: Resource   // Dropped second
    var second: Resource  // Dropped first
}
```

## Enum Drop

For enums, only the active variant's payload is dropped:

```kestrel
enum Result[T, E] {
    Ok(T)
    Err(E)
}

let r: Result[FileHandle, Error] = Result.Ok(handle)
// When r is dropped:
// - If Ok: drops FileHandle
// - If Err: drops Error
```

---

## Potential Issues

### 1. Deinit Cannot Fail

What if cleanup logic can fail?

```kestrel
struct Connection: not Copyable {
    var handle: Int
    
    deinit {
        let result = disconnect(self.handle)  // What if this fails?
        // Cannot return error or throw
    }
}
```

**Options**:
1. Deinit must be infallible (ignore errors or panic)
2. Explicit `close()` method for fallible cleanup; `deinit` as fallback
3. Allow `deinit` to throw (complex)

**Recommendation**: Provide both explicit `close() -> Result` method and infallible `deinit` fallback.

### 2. Deinit and Panics

What happens if code panics during drop?

```kestrel
func dangerous() {
    let a = Resource("a")
    let b = Resource("b")
    panic("oops")  // What about a and b?
}
```

**Options**:
1. Unwind stack, drop all values (like Rust)
2. Abort immediately, no cleanup
3. Best-effort cleanup

**Concern**: If `a.deinit` also panics during unwinding, what then? (Double panic)

### 3. Deinit Access to Self

In `deinit`, is `self` fully initialized?

```kestrel
struct Example {
    var a: Resource
    var b: Resource
    
    deinit {
        print(self.a)  // Is a still valid?
        print(self.b)  // Is b still valid?
    }
}
```

**Expected**: Yes, `self` is fully valid in `deinit`. Fields are dropped *after* the `deinit` body runs.

### 4. Cyclic References

What about reference cycles?

```kestrel
struct Node: not Copyable {
    var next: Optional[Box[Node]]
    
    deinit {
        print("dropping node")
    }
}

// Create cycle: a -> b -> a
var a = Node(next: None)
var b = Node(next: Some(Box(a)))
a.next = Some(Box(b))  // Cycle!
```

**Concern**: With cycles, neither can be dropped first.

**Mitigation**: 
- Use weak references to break cycles
- Cycles of `not Copyable` types may be statically prevented (no cycles possible without shared ownership)
- Detect cycles at runtime (expensive)

### 5. Conditional Drop

Values may or may not need dropping based on runtime conditions:

```kestrel
func example(condition: Bool) {
    let f = FileHandle(...)
    if condition {
        consume(f)  // f moved
    }
    // Is f dropped here? Only if !condition
}
```

The compiler must track "maybe moved" state and generate conditional drop code.

### 6. Drop and Generics

Generic types with `deinit`:

```kestrel
struct Wrapper[T: not Copyable] {
    var value: T
    
    deinit {
        print("dropping wrapper")
        // T's deinit is called automatically for self.value
    }
}
```

**Question**: Does `Wrapper.deinit` need to explicitly drop `value`, or is it automatic?

**Expected**: Automatic. After `deinit` body runs, fields are dropped.

### 7. Copyable Types with Deinit

Can a Copyable type have `deinit`?

```kestrel
struct Counter: Copyable {  // Explicitly Copyable
    var count: Int
    
    deinit {
        print("Counter dropped with value \(self.count)")
    }
}

let a = Counter(count: 1)
let b = a  // Copy - both have deinit
// Both a.deinit and b.deinit will be called
```

**Concern**: Copyable + deinit means deinit runs multiple times (once per copy).

**Question**: Should Copyable types with `deinit` be allowed? May lead to surprising behavior.

### 8. Deinit Order with Temporaries

When are temporaries dropped?

```kestrel
func example() {
    process(FileHandle(...))  // Temporary FileHandle
    print("after")
}
```

**Options**:
1. Drop at end of statement (after `process` returns)
2. Drop at end of scope (after `print`)

**Typical choice**: End of statement for temporaries.

### 9. Partial Initialization and Drop

If construction fails partway through:

```kestrel
struct TwoResources {
    var a: Resource
    var b: Resource
}

let x = TwoResources(
    a: Resource(...),       // Succeeds
    b: failingResource()    // Fails/panics - what about a?
)
```

**Question**: If `b`'s construction fails, is `a` dropped?

**Expected**: Yes. Partially constructed values must be cleaned up.

### 10. Deinit Visibility

Is `deinit` public or private?

```kestrel
struct Internal {
    var secret: Resource
    
    deinit {
        self.secret.cleanup()  // Accesses private field
    }
}
```

**Expected**: `deinit` has access to private fields (it's part of the type).

### 11. No Explicit Destructor Calls

Unlike C++, you cannot call `deinit` explicitly:

```kestrel
let f = FileHandle(...)
f.deinit()  // ERROR: cannot call deinit directly
```

**Mitigation**: Provide explicit `close()`, `dispose()`, or similar methods for cases where early cleanup is needed.
