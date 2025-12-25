# Execution Graph Design

The execution graph is Kestrel's mid-level intermediate representation (MIR). It sits between the semantic tree (typed AST) and LLVM IR, providing a flat, explicit representation suitable for analysis and optimization.

## Design Principles

1. **Flat namespace** - All items are fully qualified, no nesting
2. **Explicit** - Self types, generics, calling conventions all visible
3. **Place/Value distinction** - Places are memory locations, values are computed results
4. **No SSA** - Places can be reassigned (like Rust MIR)
5. **Generic** - Monomorphization happens at LLVM lowering

## Items

All items use fully qualified names. Items are declared at the top level, never nested.

### Structs

```
struct Module.Path.StructName[T, U] {
    field1: Type1
    field2: Type2
}
```

Example:
```
struct std.vec.Vec[T] {
    ptr: p[T]
    len: i64
    cap: i64
}

struct std.option.Option."cases".Some[T] {
    0: T
}

struct std.option.Option."cases".None[T] {}
```

### Enums

Enums declare their discriminants and map to case structs:

```
enum Module.Path.EnumName[T] {
    CaseName1: Module.Path.EnumName."cases".CaseName1[T]
    CaseName2: Module.Path.EnumName."cases".CaseName2[T]
}
```

Example:
```
enum std.option.Option[T] {
    Some: std.option.Option."cases".Some[T]
    None: std.option.Option."cases".None[T]
}
```

Each case has an associated struct in the `"cases"` namespace containing the payload fields.

### Protocols

```
protocol Module.Path.ProtocolName[T] {
    type AssociatedType
    func method(self: &Self, args...) -> ReturnType
}
```

Example:
```
protocol std.iter.Iterator {
    type Item
    func next(self: &var Self) -> Option[Self.Item]
}

protocol std.ops.Callable[Args, Ret] {
    func call(self: &Self, args: Args) -> Ret
}
```

### Witnesses

A witness proves that a type implements a protocol. It maps associated types and methods to concrete implementations:

```
witness Type[T]: Protocol {
    type AssociatedType = ConcreteType
    func method = path.to.implementation
}
```

Example:
```
witness std.vec.Vec[T]: std.iter.Iterator {
    type Item = T
    func next = std.vec.Vec[T].iter_next
}

witness example.main."closures".0: std.ops.Callable[(Int), Int] {
    func call = example.main."closures".0.call
}
```

### Functions

```
func Module.Path.function_name[T](param1: Type1, param2: Type2) -> ReturnType
where T: Protocol, T.Item = Int
{
    locals:
        %name: Type
        ...
    
    bb0:
        // statements
        // terminator
    
    bb1:
        // statements
        // terminator
}
```

Example:
```
func std.vec.Vec[T].push(self: &var Vec[T], value: T) -> ()
where T: Clone
{
    locals:
        %len: i64
        %slot: &var T
    
    bb0:
        %len = call std.vec.Vec[T].len(ref %self)
        %slot = call std.vec.Vec[T].get_unchecked_mut(ref var %self, %len)
        (deref %slot) = move %value
        return ()
}
```

### Static Data

Global constants and mutable statics:

```
static Module.Path.CONSTANT: Type = value
static var Module.Path.mutable_global: Type = value
```

Example:
```
static std.math.PI: f64 = f64.literal 3.14159265358979
static std.messages.GREETING: str = str.literal "Hello, world!"
static var example.counter: i64 = i64.literal 0
```

## Types

### Primitive Types

```
// Integer types
i8
i16
i32
i64

// Floating point types
f16
f32
f64

// Other primitives
bool            // Boolean
()              // Unit
!               // Never
```

### Built-in Compound Types

```
p[T]            // Pointer to T
str             // String slice: fat pointer { p[i8], i64 }
```

### Compound Types

```
// Struct/Enum types (fully qualified)
std.vec.Vec[i64]
std.option.Option[T]

// Tuple types
(i64, bool, str)

// Reference types
&T              // immutable reference
&var T          // mutable reference

// Function types
func(i64, i64) -> i64             // thin (no env, FFI-safe)
func escaping(i64, i64) -> i64    // thick (has env, can escape)
```

### Generic Types

Type parameters are preserved until LLVM lowering:

```
func identity[T](x: T) -> T { ... }
struct Box[T] { value: T }
```

## Callables (Thin vs Thick)

The execution graph distinguishes between:

- **Thin callables** (`func(Args...) -> Ret`): a bare function pointer with no environment.
- **Thick callables** (`func escaping(Args...) -> Ret`): a callable that may carry an environment (e.g. closures, bound methods).

### Calling

```
%r = call path.to.function(<value>...)        // direct call (immediate callee)
%r = call %thin_fn(<value>...)               // thin callable stored in a place
%r = call escaping %thick_fn(<value>...)     // thick callable stored in a place
```

### Passing Callables

- Use `func(Args...) -> Ret` parameters to accept only non-capturing callables.
- Use `func escaping(Args...) -> Ret` parameters to accept closures and bound methods (and plain functions via `func.to.escaping`).

### Function-to-Closure Coercion

Plain functions can be explicitly coerced to an escaping callable:

```
%thick = func.to.escaping path.to.function
```

This produces a thick callable with an empty environment.

### Partial Application

`apply partial` creates a thick callable by binding captured values:

```
%closure = apply partial path.to.function(captures...)
```

This is also how "method values" (bound methods) are represented:

```
%bound = apply partial Type.method(ref %receiver)
```

Normal method calls should stay as direct calls with an explicit receiver argument; `apply partial` is only used when a method is being treated as a value.

### Witness Methods

`witness_method Protocol.method for Type` produces an immediate referring to the selected implementation. It is a thin callable; use `func.to.escaping` when an escaping callable is required.

## Functions

### Structure

A function consists of:
1. **Signature** - name, type parameters, parameters, return type, where clause
2. **Locals block** - all local variables with explicit types
3. **Basic blocks** - labeled sequences of statements ending in a terminator

```
func example.function[T](param: T) -> T
where T: Clone
{
    locals:
        %result: T
        %temp: i64
    
    bb0:
        // statements...
        // terminator
    
    bb1:
        // statements...
        // terminator
}
```

### Locals

All local variables are declared at the top of the function with explicit types:

```
locals:
    %x: i64
    %point: Point
    %opt: Option[str]
    %ref: &var Vec[i64]
```

Locals use the `%` prefix to distinguish them from global names.

### Initializers

Initializers are regular functions that take `self: &var Self` as the first parameter and initialize it:

```
func example.Point.init(self: &var example.Point, x: i64, y: i64) -> () {
    bb0:
        (deref %self).x = move %x
        (deref %self).y = move %y
        return ()
}
```

At the call site, the caller allocates the place and passes a mutable reference:

```
locals:
    %point: example.Point

bb0:
    call example.Point.init(ref var %point, i64.literal 10, i64.literal 20)
    // %point is now initialized
```

## Places

A place is a memory location that can be read from, written to, or referenced.

### Place Expressions

```
%local                      // local variable
%local.field                // struct field projection
%local.0                    // tuple index projection
%local.CaseName             // enum case (valid after switch on that case)
%local.CaseName.field       // enum case field
deref %ref                  // dereference a reference

```

### Derived Places

Places can be derived from other places:

```
// From local
%point.x                    // field of local

// From deref
(deref %ref).field          // field of referent
(deref %ptr_ref).0          // tuple element of referent

// Chained
(deref %ref).items.0        // nested access
```

## Values

A **value** is either a **place** or an **immediate**:

```
Value = Place | Immediate
```

- **Place**: A memory location (`%local`, `%local.field`, `deref %ref`, etc.)
- **Immediate**: A constant (`i64.literal 42`, `true`, `path.to.function`, etc.)

Operations take values as operands. Results are assigned to places.

## Assignment

All assignments have the form `<place> = <rvalue>`. The left-hand side is always a place.

### Move and Copy

```
<place> = move <place>          // transfer ownership, invalidates source
<place> = copy <place>          // bitwise copy (Copy types only)
```

### Immediate Assignment

```
<place> = <immediate>           // assign constant value (literals are explicitly typed)
```

### Reference Creation

```
<place> = ref <place>           // &T (immutable borrow / reborrow)
<place> = ref var <place>       // &var T (mutable borrow / reborrow)
```

`ref` and `ref var` borrow the **referent** when their operand already has a reference type:

- If `%x: T`, then `ref %x: &T` and `ref var %x: &var T`.
- If `%r: &T`, then `ref %r: &T` (reborrow).
- If `%r: &var T`, then `ref %r: &T` and `ref var %r: &var T` (reborrow).

### Examples

```
%x = i64.literal 42             // immediate
%y = copy %x                    // copy
%z = move %y                    // move (invalidates %y)
%point.x = i64.literal 10       // field assignment
(deref %ref).y = copy %val      // through reference

%r = ref %point                 // immutable borrow
%r = ref var %point             // mutable borrow
```

## Immediates

Immediates are constant values that can be used directly as operands.

### Literal Immediates

```
i64.literal 42              // i64
f64.literal 3.14            // f64
true                        // bool
false                       // bool
```

### String Literals

String literals produce a `str` (fat pointer to static data):

```
%s = str.literal "hello"        // type: str
```

The actual bytes are embedded in the binary at codegen time.

### Function Immediates

```
path.to.function            // bare function reference
path.to.function[T]         // generic function reference
```

### Witness Method Immediates

```
witness_method Protocol.method for Type
```

Example:
```
witness_method Iterator.next for std.vec.Vec[T]
witness_method Callable.call for F
```

### Inline Declarations

Complex values can be named at point of use for readability:

```
// Naming an immediate
bb0:
    inline next = witness_method Iterator.next for I
    %opt = call next(ref var %iter)

// Naming a place
bb0:
    inline current = (deref %iter).buffer.current
    %val = copy current
    current = move %new_val
```

Inlines are scoped to the rest of the function from their declaration point. They do not use the `%` prefix. An inline place is an alias for the place expression - the same validity rules apply (e.g., moving out of it invalidates it).

`inline` introduces no new core IR operation; it is pure syntax sugar that can be desugared away after parsing.

## Statements

Statements are the operations within a basic block. Each basic block ends with exactly one terminator.

Operands to statements are **values** (places or immediates).

### Arithmetic

```
// Integer (signed/unsigned variants)
%r = i64.add.signed <value>, <value>
%r = i64.sub.signed <value>, <value>
%r = i64.mul.signed <value>, <value>
%r = i64.div.signed <value>, <value>
%r = i64.rem.signed <value>, <value>
%r = i64.neg <value>

// Floating point
%r = f64.add <value>, <value>
%r = f64.sub <value>, <value>
%r = f64.mul <value>, <value>
%r = f64.div <value>, <value>
%r = f64.neg <value>
```

Examples:
```
%r = i64.add.signed %a, %b              // two places
%r = i64.add.signed %a, i64.literal 1   // place + immediate
%r = i64.add.unsigned %a, %b            // unsigned variant
%r = f64.mul %x, f64.literal 2.0        // floating point
```

### Bitwise

```
%r = i64.and <value>, <value>
%r = i64.or <value>, <value>
%r = i64.xor <value>, <value>
%r = i64.shl <value>, <value>
%r = i64.shr.signed <value>, <value>
%r = i64.shr.unsigned <value>, <value>
%r = i64.not <value>
```

### Comparison

```
// Integer (lt/le/gt/ge have signed/unsigned variants)
%r = i64.eq <value>, <value>
%r = i64.ne <value>, <value>
%r = i64.lt.signed <value>, <value>
%r = i64.le.signed <value>, <value>
%r = i64.gt.signed <value>, <value>
%r = i64.ge.signed <value>, <value>

// Floating point
%r = f64.eq <value>, <value>
%r = f64.ne <value>, <value>
%r = f64.lt <value>, <value>
%r = f64.le <value>, <value>
%r = f64.gt <value>, <value>
%r = f64.ge <value>, <value>
```

### Logical

```
%r = bool.and <value>, <value>
%r = bool.or <value>, <value>
%r = bool.not <value>
```

### Construction

```
// Struct (fields take values)
%r = construct Point { x: <value>, y: <value> }

// Enum variant
%r = construct Option[i64].Some { 0: <value> }
%r = construct Option[i64].None {}

// Tuple
%r = construct (i64, bool) { 0: <value>, 1: <value> }
```

Examples:
```
%r = construct Point { x: %xval, y: %yval }     // places
%r = construct Point { x: i64.literal 10, y: i64.literal 20 }   // immediates
%r = construct Option[i64].Some { 0: %val }
```

### Function Calls

Arguments are values (places or immediates):

```
// Direct call to known function (function is immediate)
%r = call path.to.function(<value>, <value>)
call path.to.function(<value>)           // unit return

// Call with generic params
%r = call path.to.function[T, U](<value>)

// Call thin function pointer (function is place)
%r = call %thin_fn(<value>)

// Call escaping function (thick, has environment)
%r = call escaping %closure(<value>)
```

Examples:
```
%r = call example.add(%x, %y)           // places
%r = call example.add(%x, i64.literal 1)                        // place + immediate
%r = call example.add(i64.literal 1, i64.literal 2)             // immediates
call std.io.print(%msg)                 // unit return
```

### Partial Application

Creates an escaping function (thick) from a function and captured environment:

```
%closure = apply partial path.to.function(captures...)
```

This is used to form closures and bound methods. Normal method calls should be direct calls with an explicit receiver argument (not `apply partial`).

Example:
```
// Creating a closure
%env = construct example.main."closures".0 { x: copy %x }
%closure = apply partial example.main."closures".0.call(ref %env)

// Binding a method to receiver
%bound = apply partial Point.magnitude(ref %point)
```

### Type Conversions

```
// Integer/float conversions
%f = i64.to.f64 <value>
%i = f64.to.i64 <value>
%f32 = f64.to.f32 <value>

// Integer widening/truncation
%big = i32.to.i64 <value>
%small = i64.to.i32 <value>

// Pointer casts
%ptr = ptr.bitcast[p[U]] <value>

// Reference coercion (&var T -> &T)
%immut = ref.to.immut <value>
```

Examples:
```
%f = i64.to.f64 %i              // place
%f = i64.to.f64 i64.literal 42  // immediate
```

### String Operations

```
// Create string from literal (produces str)
%s = str.literal "hello"

// Extract parts from str
%ptr = str.ptr <value>          // str -> p[i8]
%len = str.len <value>          // str -> i64

// Construct str from parts
%s = str.from_parts <value>, <value>   // p[i8], i64 -> str
```

### Pointer Operations

```
// Null pointer (immediate)
%ptr = ptr.null[T]              // p[T]

// Pointer offset
%ptr2 = ptr.offset <value>, <value>   // p[T], i64 -> p[T]

// Pointer to reference (unsafe)
%ref = ptr.to.ref <value>       // p[T] -> &T
%ref = ptr.to.ref_var <value>   // p[T] -> &var T

// Reference to pointer
%ptr = ref.to.ptr <value>       // &T -> p[T]
```

## Terminators

Each basic block ends with exactly one terminator.

### Return

```
return <value>          // return value
return ()               // return unit
```

### Jump

Unconditional branch:

```
jump bb1
```

### Branch

Conditional branch on boolean:

```
branch if <value>, bb_true else bb_false
```

### Switch

Branch on enum discriminant (must be a place):

```
switch <place> {
    Case1 => bb1
    Case2 => bb2
}
```

After a switch, the corresponding case's fields are accessible:

```
bb0:
    switch %opt {
        Some => bb1
        None => bb2
    }

bb1:
    %value = move %opt.Some.0   // valid: we know it's Some
    ...
```

### Panic

Abort execution with message (no unwinding):

```
panic "error message"
```

### Unreachable

Marks statically impossible paths:

```
unreachable
```

## Comments

Line comments start with `//`:

```
// This is a comment
%x = i64.add.signed %a, %b  // inline comment
```

Block comments are not supported.

## Lowering from Semantic Tree

### Variable Declarations

```kestrel
// Source
let x: Int = 42
var y: Int = 0
```

```
// Execution Graph
locals:
    %x: i64
    %y: i64

bb0:
    %x = i64.literal 42
    %y = i64.literal 0
```

### Assignments

```kestrel
// Source
y = x + 1
point.x = 10
```

```
// Execution Graph
bb0:
    %tmp = i64.add.signed %x, i64.literal 1
    %y = move %tmp
    %point.x = i64.literal 10
```

### If Expressions

```kestrel
// Source
let result = if condition { a } else { b }
```

```
// Execution Graph
locals:
    %result: i64

bb0:
    branch if %condition, bb_then else bb_else

bb_then:
    %result = copy %a
    jump bb_join

bb_else:
    %result = copy %b
    jump bb_join

bb_join:
    // %result is now valid
```

### While Loops

```kestrel
// Source
while condition {
    body
}
```

```
// Execution Graph
bb_loop_header:
    branch if %condition, bb_loop_body else bb_loop_exit

bb_loop_body:
    // body statements
    jump bb_loop_header

bb_loop_exit:
    // continue
```

### Loop with Break/Continue

```kestrel
// Source
loop {
    if done { break }
    continue
}
```

```
// Execution Graph
bb_loop:
    branch if %done, bb_exit else bb_continue

bb_continue:
    jump bb_loop

bb_exit:
    // after loop
```

### Function Calls

```kestrel
// Source
let result = add(x, y)
```

```
// Execution Graph
%result = call example.add(%x, %y)
```

### Method Calls

```kestrel
// Source
let len = vec.len()
vec.push(42)
```

```
// Execution Graph
%len = call std.vec.Vec[i64].len(ref %vec)
call std.vec.Vec[i64].push(ref var %vec, i64.literal 42)
```

### Struct Instantiation

```kestrel
// Source
let point = Point(x: 10, y: 20)
```

```
// Execution Graph (with explicit init)
locals:
    %point: example.Point

bb0:
    call example.Point.init(ref var %point, i64.literal 10, i64.literal 20)

// Or direct construction (memberwise init)
%point = construct example.Point { x: i64.literal 10, y: i64.literal 20 }
```

### Field Access

```kestrel
// Source
let x = point.x
point.y = 30
```

```
// Execution Graph
%x = copy %point.x
%point.y = i64.literal 30
```

### Match Expressions

```kestrel
// Source
match opt {
    Some(x) => x + 1
    None => 0
}
```

```
// Execution Graph
locals:
    %result: i64

bb0:
    switch %opt {
        Some => bb_some
        None => bb_none
    }

bb_some:
    %x = move %opt.Some.0
    %result = i64.add.signed %x, i64.literal 1
    jump bb_join

bb_none:
    %result = i64.literal 0
    jump bb_join

bb_join:
    // %result is valid
```

### Closures

```kestrel
// Source
let x = 10
let add_x = { y in x + y }
let result = add_x(5)
```

```
// Execution Graph

struct example.main."closures".0 {
    x: i64
}

func example.main."closures".0.call(env: &example.main."closures".0, y: i64) -> i64 {
    locals:
        %result: i64
    
    bb0:
        %result = i64.add.signed (deref %env).x, %y
        return %result
}

func example.main() -> () {
    locals:
        %x: i64
        %env: example.main."closures".0
        %add_x: func escaping(i64) -> i64
        %result: i64
    
    bb0:
        %x = i64.literal 10
        %env = construct example.main."closures".0 { x: copy %x }
        %add_x = apply partial example.main."closures".0.call(ref %env)
        %result = call escaping %add_x(i64.literal 5)
        return ()
}
```

### Generic Functions

```kestrel
// Source
func identity[T](x: T) -> T { x }
let n = identity(42)
```

```
// Execution Graph (keeps generics)
func example.identity[T](x: T) -> T {
    locals:
        %result: T
    
    bb0:
        %result = copy %x
        return %result
}

func example.main() -> () {
    locals:
        %n: i64
    
    bb0:
        %n = call example.identity[i64](i64.literal 42)
        return ()
}
```

### Protocol Methods

```kestrel
// Source
func process[I](iter: &var I) -> Int 
where I: Iterator, I.Item = Int
{
    let opt = iter.next()
    // ...
}
```

```
// Execution Graph
func example.process[I](iter: &var I) -> i64
where I: Iterator, I.Item = i64
{
    locals:
        %opt: Option[i64]
    
    bb0:
        inline next = witness_method Iterator.next for I
        %opt = call next(%iter)
        // ...
}
```

### Extensions

Extensions don't exist as items - their methods are lifted to top-level functions:

```kestrel
// Source
extend Int {
    func double(self) -> Int { self * 2 }
}
```

```
// Execution Graph
func i64.double(self: i64) -> i64 {
    locals:
        %result: i64
    
    bb0:
        %result = i64.mul.signed %self, i64.literal 2
        return %result
}
```

## Full Example

```kestrel
// Source
module example

struct Counter {
    var count: Int
    
    init() {
        self.count = 0
    }
    
    mutating fn increment(self) {
        self.count = self.count + 1
    }
    
    func get(self) -> Int {
        self.count
    }
}

func main() {
    var counter = Counter()
    counter.increment()
    counter.increment()
    let value = counter.get()
}
```

```
// Execution Graph

struct example.Counter {
    count: i64
}

func example.Counter.init(self: &var example.Counter) -> () {
    bb0:
        (deref %self).count = i64.literal 0
        return ()
}

func example.Counter.increment(self: &var example.Counter) -> () {
    locals:
        %tmp: i64
    
    bb0:
        %tmp = copy (deref %self).count
        %tmp = i64.add.signed %tmp, i64.literal 1
        (deref %self).count = move %tmp
        return ()
}

func example.Counter.get(self: &example.Counter) -> i64 {
    locals:
        %result: i64
    
    bb0:
        %result = copy (deref %self).count
        return %result
}

func example.main() -> () {
    locals:
        %counter: example.Counter
        %value: i64
    
    bb0:
        call example.Counter.init(ref var %counter)
        call example.Counter.increment(ref var %counter)
        call example.Counter.increment(ref var %counter)
        %value = call example.Counter.get(ref %counter)
        return ()
}
```

## Summary

| Concept | Syntax |
|---------|--------|
| Struct | `struct Path.Name[T] { field: Type }` |
| Enum | `enum Path.Name[T] { Case: CaseStruct }` |
| Protocol | `protocol Path.Name { type T; func method(...) }` |
| Witness | `witness Type: Protocol { type T = ...; func m = ... }` |
| Function | `func Path.name[T](params) -> Ret where ... { locals: ... bb0: ... }` |
| Static | `static Path.NAME: Type = value` |
| Local | `%name` |
| Inline | `inline name = <immediate or place>` |
| Place | `%local`, `%local.field`, `deref %ref` |
| Move | `<place> = move <place>` |
| Copy | `<place> = copy <place>` |
| Ref | `<place> = ref <place>`, `<place> = ref var <place>` |
| Assign | `<place> = <immediate>` |
| Call | `call path.to.func(args)`, `call %thin(args)`, `call escaping %thick(args)` |
| Callable coercion | `%thick = func.to.escaping path.to.function` |
| Partial apply | `%c = apply partial func(captures)` |
| Construct | `%v = construct Type { field: %val }` |
| Conversion | `i64.to.f64 %v`, `ptr.bitcast[p[T]] %v`, `ref.to.immut %r` |
| String | `str.literal "..."`, `str.ptr %s`, `str.len %s`, `str.from_parts %p, %l` |
| Pointer | `ptr.null[T]`, `ptr.offset %p, %n`, `ptr.to.ref %p`, `ref.to.ptr %r` |
| Return | `return %val`, `return ()` |
| Jump | `jump bb1` |
| Branch | `branch if %cond, bb_t else bb_f` |
| Switch | `switch %e { Case => bb }` |
| Panic | `panic "message"` |
| Unreachable | `unreachable` |
