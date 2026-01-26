# Kestrel Language Roadmap

## Phase 1: Type System Foundation

### Core Type System

- [x] Type Aliases - Define reusable type names (`type String = Array<Char>`)
  - [x] Parser support
  - [x] Semantic tree representation (TypeAliasSymbol)
  - [x] Type resolution (resolves aliased types)
  - [x] Circular alias detection
- [x] Import Resolution - Complete the ImportResolver
  - [x] Module path resolution
  - [x] Imported symbol lookup
  - [x] Specific imports `import A.(Foo, Bar)`
  - [x] Aliased imports `import A as B`, `import A.(Foo as F)`
  - [x] Whole-module imports `import A.B.C`
  - [x] Visibility checking (public/private/internal/fileprivate)
  - [x] Cross-file error reporting with precise spans
- [x] Type Resolution - Resolve type references across modules
  - [x] Path type resolution (`A.B.C` -> concrete type)
  - [x] Scope-aware name lookup
  - [x] Cross-module type references
- [x] Primitive Types - Int, Float, String, Bool (TyKind variants)

### Modules & Visibility (moved from Phase 5 - DONE)

- [x] Module declarations (`module A.B.C`)
- [x] Visibility modifiers (public, private, internal, fileprivate)
- [x] Visibility scope tracking
- [x] Cross-module visibility checking

### Classes (Temporary - will be replaced by Structs)

- [x] Class declarations with visibility
- [x] Nested classes
- [x] Class type representation

### Structured Types

- [x] Structs - Replace classes with lightweight data structures
  - [x] Parser support for struct declarations
  - [x] Semantic tree representation (StructSymbol)
  - [x] Struct type resolution (TyKind::Struct)
- [x] Struct Fields / Global Variables - `(visibility)? (static)? let/var name: Type`
  - [x] Parser support for field declarations
  - [x] Semantic tree representation (FieldSymbol)
  - [x] Static vs instance field tracking
  - [x] Mutability (let vs var)
  - [x] Works in struct bodies and at module level (globals)
- [x] Protocols - Define interfaces/contracts
  - [x] Parser support for protocol declarations
  - [x] Semantic tree representation (ProtocolSymbol)
  - [x] Protocol type resolution (TyKind::Protocol)
  - [x] Generic protocols with type parameters and where clauses
  - [x] Validation: protocol methods cannot have bodies
  - [x] Protocol inheritance (`protocol A: B { }`)
  - [x] Protocol conformance syntax (`struct Point: Drawable { }`)
  - [x] Conformance validation (check all methods implemented)
  - [x] Protocol initializer declarations (`init()` in protocols)

### Functions

- [x] Function Declarations - `(visibility)? (static)? fn name(params) (-> Type)? { }`
  - [x] Parser support for function declarations
  - [x] Function signatures with parameter types
  - [x] Return type declarations
  - [x] Labeled parameters (`fn greet(with name: String)`)
  - [x] Semantic tree representation (FunctionSymbol)
  - [x] CallableBehavior for callable semantics
- [x] Function Overloading
  - [x] Overloading by arity (different parameter counts)
  - [x] Overloading by parameter types
  - [x] Overloading by labels (labeled vs unlabeled)
  - [x] Duplicate signature detection with clear error messages
- [x] Function Types - First-class function types `(Int, Int) -> Int`
  - [x] Parser support for function type syntax
  - [x] TyKind::Function representation

### Type Expressions (Parser)

- [x] Unit type `()`
- [x] Never type `!`
- [x] Tuple types `(T1, T2, ...)`
- [x] Function types `(P1, P2) -> R`
- [x] Path types `A.B.C`

## Phase 2: Generics

- [x] Generic Type Parameters - `Struct[T]`, `Protocol[T]`
  - [x] Parser support for type parameter syntax
  - [x] TypeParameterSymbol representation
  - [x] Type parameter defaults `[T = Int]`
  - [x] Type argument application and arity checking
- [x] Generic Functions - `func identity[T](value: T) -> T`
  - [x] Parser support
  - [x] FunctionSymbol with type parameters
- [x] Generic Constraints - `where T: Protocol`
  - [x] Parser support for where clauses
  - [x] WhereClause representation with bounds
  - [x] Validation (bounds must be protocols, params must exist)
- [x] Type Substitutions - Replace type parameters with concrete types
  - [x] Substitutions system for generic instantiation
  - [x] Recursive substitution through complex types

## Phase 3: Values & Expressions

### Literals

- [x] Integer Literals - `42`, `0xFF`, `0b1010`
- [x] Float Literals - `3.14`, `1.0e10`
- [x] String Literals - `"hello"`, escape sequences
- [x] Bool Literals - `true`, `false`
- [x] Array Literals - `[1, 2, 3]`
- [x] Tuple Literals - `(1, 2, 3)`

### Paths

- [x] Paths - `a.b.c`
- [x] Resolving paths - resolve to a value
- [x] Symbols can have a value associated with them (ValueBehavior)

### Variables

- [x] Variable Declarations - `let x: Int = 42`
- [x] Mutable Variables - `var x: Int = 42`
- [x] Pattern-based bindings (Statement::Binding with Pattern)
- [x] Assignment Expressions - `x = 43`, `point.x = 10`
  - [x] Parser: `=` operator (lowest precedence, right-associative)
  - [x] AST: `ExprKind::Assignment { target, value }`
  - [x] Type: Returns `Never` (assignment as expression)
  - [x] Validation: Mutability checking for variables and fields
  - [x] Expression mutability tracking on all expressions

### Function Operations

- [x] Function Calls - `add(1, 2)`, `module.function(arg)`
- [x] Calling overloaded functions (by arity + labels)
- [x] Method Calls - `obj.method(args)`
- [x] Primitive Method Calls - `5.toString()`, `"hello".length()`
- [x] Self Parameter Handling - Methods get `self` automatically
  - [x] ReceiverKind enum (Borrowing, Mutating, Consuming, Initializing)
  - [x] `mutating func` and `consuming func` syntax
  - [x] Auto-injection of `self` local in instance methods
  - [x] Self type resolution for member access
  - [x] Error for `self` in static methods and free functions
- [x] Call validation
  - [x] Error for undefined functions
  - [x] Error for wrong arity in calls
  - [x] Error for wrong labels in calls
  - [x] Error for calling instance method on type name

### Expressions

- [x] Member Access - `struct.field` (via MemberAccessBehavior)
- [x] Chained Member Access - `obj.method().field` (parser fix for postfix member access)
- [x] Binary Operators - `+`, `-`, `*`, `/`, `%`, `&`, `|`, `^`, `<<`, `>>`
  - [x] Pratt parsing for precedence handling
  - [x] Desugar to method calls (`a + b` → `a.add(b)`)
  - [x] Primitive method lookup (Int, Float)
- [x] Comparison Operators - `==`, `!=`, `<`, `>`, `<=`, `>=`
  - [x] Desugar to `eq`, `ne`, `lt`, `gt`, `le`, `ge` methods
  - [x] Primitive methods for Int, Float, Bool, String
- [x] Logical Operators - `and`, `or`, `not`
  - [x] Desugar to `logicalAnd`, `logicalOr`, `logicalNot` methods
  - [x] Primitive methods on Bool type
- [x] Unary Operators - `-x`, `+x`, `!x`, `not x`, `x!`
  - [x] Prefix: `neg`, `identity`, `bitNot`, `logicalNot`
  - [x] Postfix: `unwrap` (for optionals, not yet implemented)

### Struct Operations

- [x] Struct Instantiation - `Point(x: 10, y: 20)`
  - [x] Implicit memberwise initializer (generated from fields)
  - [x] Labeled argument matching (field names in declaration order)
  - [x] TypeRef expression for struct names as callees
  - [x] Diagnostics for arity/label mismatches
- [x] Struct Initializers - `init() {}`
  - [x] Parser support for initializer declarations
  - [x] InitializerSymbol with CallableBehavior
  - [x] ReceiverKind::Initializing for self handling
  - [x] Explicit init suppresses implicit memberwise init
  - [x] Initializer body resolution with field initialization verification
- [x] Field Access - `point.x`, `point.y`
- [x] Field Assignment - `point.x = 10` (with mutability validation)

## Phase 4: Control Flow

- [x] If Expressions - `if condition { ... } else { ... }`
  - [x] Parser support for if/else/else-if chains
  - [x] Semantic tree representation (ExprKind::If)
  - [x] If as expression (returns value from branches)
  - [x] Condition must be Bool
- [x] While Loops - `while condition { ... }`
  - [x] Parser support with optional labels (`label: while ...`)
  - [x] Semantic tree representation (ExprKind::While)
  - [x] Labeled loops for break/continue targets
- [x] Loop - `loop { ... }`
  - [x] Parser support with optional labels
  - [x] Infinite loop (exits via break or return)
- [x] Break/Continue - `break`, `break label`, `continue`, `continue label`
  - [x] Parser support for labeled break/continue
  - [x] Semantic validation (must be inside loop)
  - [x] Label resolution to target loop
- [x] Return - `return`, `return expr`
  - [x] Parser support for return with optional value
  - [x] Semantic tree representation (ExprKind::Return)
  - [x] Type is Never (control transfers out)

## Phase 5: Validation & Type Checking

- [x] Initializer Verification - Field initialization analysis
  - [x] All fields must be initialized before return
  - [x] `let` fields can only be assigned once
  - [x] Fields cannot be read before assigned
  - [x] Control flow analysis (if/else, loops, return)
- [x] Dead Code Detection - Unreachable code warnings
  - [x] Code after return
  - [x] Code after break/continue
  - [x] Code after infinite loops
- [x] Exhaustive Return Analysis - All paths must return
  - [x] Functions with non-unit return types checked
  - [x] Control flow analysis for all code paths
  - [x] Handles if/else, loops, early returns
- [x] Never Type Propagation
  - [x] Expressions containing Never propagate correctly
  - [x] Type compatibility with Never (Ty::join)
- [x] Type Checking - Full type validation
  - [x] Return type checking (return expr matches declared type)
  - [x] Assignment type checking
  - [x] Variable binding type checking
  - [x] Function/initializer argument type checking
  - [x] If/while condition must be Bool
  - [x] If branch types must match (when used as expression)
  - [x] Array element types must be consistent
  - [x] Struct nominal equality (different structs are incompatible)
  - [x] Generic struct type inference in implicit init
- [x] Tuple Indexing - `tuple.0`, `tuple.1`
  - [x] Parser support for integer member access
  - [x] Semantic validation (index in bounds)
  - [x] Type resolution (element type at index)
  - [x] Mutability support (assignment to tuple elements)
  - Note: Chained access (`t.0.1`) requires intermediate variables due to lexer ambiguity

## Phase 6: Generics & Protocols

- [x] Generic Constraint Enforcement ✓
  - [x] Modify `get_type_container()` to handle TypeParameter via protocol bounds
  - [x] Collect methods from all protocol bounds
  - [x] Self substitution (Self → receiver type)
  - [x] Handle ambiguous methods across multiple bounds
  - [x] Protocol inheritance chain traversal
  - [x] Call-site constraint verification
  - [x] New diagnostics for constraint errors
- [x] Static Methods on Type Parameters ✓
  - [x] Call static methods on type parameters: `T.staticMethod()`
  - [x] Call initializers on type parameters: `T()`
  - [x] Lookup methods/inits from protocol bounds (including inherited)
  - [x] Ambiguity detection across multiple bounds
  - [x] Type parameter validation (cannot be used as standalone values)
- [x] GenericsBehavior Refactor ✓
  - [x] Created GenericsBehavior for type_parameters + where_clause
  - [x] Eliminated RwLock<WhereClause> mutation pattern
  - [x] Resolvers add GenericsBehavior during BIND with resolved bounds
  - [x] Fallback to children for BUILD phase type parameter access
- [x] Associated Types ✓
  - [x] Protocol associated type declarations (`protocol Iterator { type Item }`)
  - [x] AssociatedTypeSymbol representation
  - [x] Qualified type path resolution (`T.Item`, `C.Iter.Item`)
  - [x] Associated type resolution in conforming types
  - [x] Associated type constraints (`where T.Item: Equatable`)
  - [x] Qualified bindings for disambiguation (`type Iterator.Item = Int`)
  - [x] Protocol inheritance with associated type constraints
  - [x] Default associated types with override support
  - [x] Constraint satisfaction validation
- [x] Protocol Method Linking
  - [x] Track which protocol a method satisfies when struct conforms
  - [x] Resolve protocol method calls to concrete implementations
  - [x] ProtocolImplementationBehavior for method bindings
- [x] Extensions with Conformances ✓
  - [x] `extend Type: Protocol { ... }` syntax (lexer + parser)
  - [x] ExtensionSymbol and ExtensionTargetBehavior
  - [x] Extension registry (HashMap by target type)
  - [x] ExtensionResolver (BUILD + BIND phases)
  - [x] Methods in extension satisfy protocol requirements
  - [x] Retroactive conformance (add conformance to types you don't own)
  - [x] Extension method resolution (find methods in extensions)
  - [x] Type parameter substitution in extension methods (self.field resolves correctly)
  - [x] Generic extensions (`extend Box[T]` works)
  - [x] Specialized extensions (`extend Box[Int]` works)
  - [x] Extension applicability with where clause constraints
  - [x] Specialized extension priority (Box[Int] wins over Box[T])
  - [x] Static methods in extensions
  - [x] Conflict detection (struct vs extension, extension vs extension)
  - [x] Private method visibility in extensions
  - [x] Generic type inference for extension-conforming types
  - [x] Type parameter position validation (swapped params rejected)
- [x] Tighter Type Parameter Assignability
  - [x] Type parameters only assignable to themselves (same SymbolId)
  - [x] `T` not assignable to `U`, `T` not assignable to `Int`
  - [x] Substitutions stored in Call expression for type checking
  - [x] Self substitution for protocol method calls
  - [x] Generic struct field access applies substitutions
- [x] Where Clause Equality Constraints ✓
  - [x] `TypeEquality` variant in `WhereClause::Constraint`
  - [x] Extract equality constraints from syntax (`where T.Item = Int`)
  - [x] Type checking consults where clause for equality
  - [x] Support `T = U`, `T.Item = Int`, `T.Item = U.Item`

## Phase 7: Type Inference

- [x] Local Type Inference ✓
  - [x] `let x = 42` infers `Int`
  - [x] `let p = Point(x: 1, y: 2)` infers `Point`
- [x] Generic Type Argument Inference ✓
  - [x] Infer type arguments from call arguments
  - [x] `identity(42)` infers `identity[Int](42)`
- [x] Bidirectional Type Checking ✓
  - [x] Expected type propagation into expressions
  - [x] Foundation for closure parameter inference
- [x] Static Method Type Parameter Substitution ✓
  - [x] `Box[Int].wrap(42)` substitutes `T` with `Int` in static methods
  - [x] Applies to static methods in both structs and extensions
- [x] Generic Method Type Parameter Substitution ✓
  - [x] `wrapper.rewrap[U]("hello")` infers `U` from argument
  - [x] Extension methods with their own type parameters
- [x] Constraint-based Type Inference ✓
  - [x] Hindley-Milner style solver in `kestrel-semantic-type-inference`
  - [x] Type-directed member resolution and associated type resolution
- [x] Extension Specialization Overlap Detection ✓
  - [x] Allow non-overlapping specialized extensions (`Box[Int]` vs `Box[String]`)
  - [x] Only reject truly ambiguous cases

## Phase 8: Closures & First-Class Functions

- [x] Closure Expressions
  - [x] Closure syntax (e.g., `{ x, y in x + y }` or `{ body }`)
  - [x] Capturing variables from enclosing scope (immutable by-value captures)
  - [x] Capture semantics (by value vs by reference)
- [x] Function References
  - [x] Reference named functions as values
  - [x] Pass functions to higher-order functions
- [x] Closure Type Inference
  - [x] Infer parameter types from context
  - [x] `numbers.map({ n in n * 2 })` infers `n: Int`
  - [x] Implicit `it` parameter for single-parameter closures
- [x] Trailing Closure Syntax
  - [x] Swift-style trailing closures
  - [x] Multiple trailing closures with labels

## Phase 9: Enums & Algebraic Data Types

- [x] Enum Declarations
  - [x] Simple enums: `enum Color { case Red, Green, Blue }`
  - [x] Enums with associated values: `enum Option[T] { case Some(T), None }`
  - [x] Recursive enums with `indirect` keyword
  - [x] Indirect recursion detection through structs
  - [x] Generic enums with type parameters and where clauses
  - [x] Enum instantiation (full path `Color.Red` and shorthand `.Red`)
  - [x] Protocol conformance for enums
  - [x] Instance methods in enums
  - [x] Static methods in enums
  - [x] Enum extensions (`extend Color { ... }`)
- [x] Pattern Matching
  - [x] `match` expressions
  - [x] Exhaustiveness checking
  - [x] Patterns: literals, bindings, enum variants, wildcards
  - [x] Guard clauses in patterns
  - [x] `if let` / `guard let`

## Phase 10: Execution Graph

- [x] Execution Graph IR
  - [x] Basic block representation (BasicBlock with statements + terminator)
  - [x] Control flow graph structure (FunctionDef with blocks, entry_block)
  - [x] Lower semantic-tree to execution-graph (kestrel-execution-graph-lowering)
  - [x] MirContext with arenas for all items (structs, enums, protocols, witnesses, functions, statics)
  - [x] Type interning and name interning
- [x] Type System
  - [x] Primitives: I8-I64, F16-F64, Bool, Unit, Never, Str
  - [x] Pointers/References: Pointer, Ref, RefMut
  - [x] Compound: Tuple, Array, Named (structs/enums with type args)
  - [x] Function types: FuncThin (no captures), FuncThick (closures)
  - [x] Generics: TypeParam, SelfType, AssociatedTypeProjection
- [x] Operations
  - [x] Primitive operations (arithmetic, comparisons, bitwise, boolean)
  - [x] Memory operations (Place with Local, Field, Index, Deref, Downcast)
  - [x] Control flow operations (Return, Jump, Branch, Switch, Panic, Unreachable)
  - [x] Call operations (Direct, Thin function pointer, Thick closure, Witness protocol method)
  - [x] Struct construction, tuple/array creation, enum variant creation
  - [x] Cast operations (int/float conversions, pointer casts)
  - [x] String operations (StrPtr, StrLen, StrFromParts)
  - [x] Pointer operations (PtrOffset, PtrToRef, RefToPtr)
  - [x] Closure operations (FuncToEscaping, ApplyPartial)
- [x] Item Lowering
  - [x] Functions with body, params, type params, where clauses
  - [x] Initializers (as functions with `self: &var Type`)
  - [x] Structs with fields and generic type support
  - [x] Enums with cases and payload structs
  - [x] Protocols with associated types, methods, parent protocols
  - [x] Witnesses auto-generated from conformances
  - [x] Extensions (methods lowered, witnesses generated)
- [x] Expression Lowering
  - [x] Literals (unit, int, float, bool, string)
  - [x] Variable references and field access
  - [x] Assignment
  - [x] All primitive method calls (arithmetic, comparison, bitwise, boolean)
  - [x] Struct construction
  - [x] Function/method calls (direct, witness, thick/thin)
  - [x] Control flow (if/else, if-let, while, while-let, loop)
  - [x] Break, continue, return
  - [x] Match expressions (full decision tree compilation)
  - [x] Closures (capturing and non-capturing)
  - [x] Arrays, tuples
- [x] Pattern Lowering
  - [x] Irrefutable patterns (let bindings): local, wildcard, tuple, struct, enum, array prefix, @
  - [x] Refutable patterns (match/if-let): enum variants, literals, ranges via decision trees
- [x] Pass System
  - [x] MirPass trait for whole-context passes
  - [x] FunctionPass trait for per-function passes
  - [x] PassManager with sequential execution
  - [x] Fixed-point iteration support
- [x] Additional Features
  - [x] Method references as values (bound methods)
  - [x] Int.toString() (IntToString MIR operation)

## Phase 11: Memory Model ✅ COMPLETE

See [docs/memory-model/implementation-plan.md](docs/memory-model/implementation-plan.md) for detailed implementation plan.

### Phase 11.1: Parameter Access Modes + MIR Foundation ✅

- [x] Parser: `consuming`/`mutating` keywords on parameters
- [x] Semantic model: `AccessMode` enum (Borrow, Mutating, Consuming)
- [x] Call-site validation: `mutating` requires `var`, track moved variables
- [x] MIR: `PassingMode` enum (Ref, MutRef, Copy, Move) on Call args
- [x] Diagnostics: "cannot pass let to mutating", "use of moved value"

### Phase 11.2: Attributes ✅

- [x] Parser: `@attribute` and `@attribute(args)` syntax
- [x] Semantic model: `AttributesBehavior` on all declarations
- [x] Known attributes: `@builtin(.Feature)` for language features

### Phase 11.3: Builtin Protocols ✅

- [x] `@builtin(.Copyable)` protocol for implicit copy semantics
- [x] `BuiltinRegistry` for tracking language feature protocols
- [x] Validation: marker protocol requirements, duplicate detection

### Phase 11.4: Copyable / not Copyable ✅

- [x] Parser: `not Copyable` in struct/enum conformance list
- [x] Semantic model: `CopySemantics` (Copyable, Cloneable, NotCopyable) on structs/enums
- [x] Inference: not Copyable if any field is not Copyable
- [x] Move tracking: use-after-move errors for not Copyable types
- [x] MIR: emit Copy vs Move based on type's CopySemantics

### Phase 11.5: Drop Semantics (RAII) ✅

- [x] Parser: `deinit { }` blocks in structs
- [x] Semantic model: `DeinitSymbol`, at most one per struct
- [x] MIR: `Deinit` instruction, insert at scope exit (reverse order)
- [x] Conditional drops for maybe-moved variables (`DeinitIf` with drop flags)
- [x] `deinit x;` statement for early drop
- [x] Temporaries drop at end of statement
- [x] Struct field drops in reverse order, enum variant drops via switch

### Phase 11.6: Cloneable Protocol ✅

- [x] Define `@builtin(.Cloneable)` protocol inheriting from `Copyable`
- [x] `@builtin(.Clone)` on `clone(self) -> Self` method
- [x] For Cloneable types, copy emits witness call to `clone()`
- [x] Cloneable field propagation (struct with Cloneable field must conform)
- [x] Conflicting conformance detection (`Cloneable + not Copyable` is error)

### Phase 11.7: Generics Integration ✅

- [x] Parser: `where T: not Copyable` syntax in where clauses
- [x] Default `[T]` = `[T: Copyable]` (can copy T values)
- [x] `where T: not Copyable` relaxes bound (cannot copy, only move)
- [x] Context-aware copyability checking in body resolution

### Future Work (Not Planned)

- [ ] Conditional conformance: `Box[T]` Copyable iff `T` Copyable
- [ ] Existential types: `any Protocol` syntax, dynamic dispatch

## Phase 12: Code Generation

- [ ] IR Generation
  - [ ] Choose target: LLVM, WASM, bytecode, or transpile
- [ ] Runtime Support
  - [ ] Memory management implementation
  - [ ] Built-in function implementations
- [ ] Executable Output
  - [ ] Binary or interpreted execution

## Phase 13: Standard Library & Language Features ✅ COMPLETE

### Computed Properties & Subscripts

- [x] Computed properties with getter/setter
- [x] Shorthand syntax: `var x: Int { expr }`
- [x] Explicit accessors: `var x: Int { get { expr } set { expr } }`
- [x] Protocol requirements: `{ get }` or `{ get set }`
- [x] Subscripts with `subscript[T]?(params) -> Type { body }`
- [x] `ExprKind::SubscriptCall` for `receiver[args]` expressions

### Protocol Extensions & Operators

- [x] Protocol extensions with default implementations
- [x] `extend Protocol { ... }` syntax
- [x] `Constraint::SelfBound` for conditional extensions
- [x] Protocol operators (58 operator protocols)
- [x] Operators desugar to protocol method calls

### Try Operator & Error Handling

- [x] `try expr` syntax with high precedence
- [x] Desugars to match on `tryExtract()` method
- [x] `Tryable` and `FromResidual` protocols
- [x] `ControlFlowEnum` with `Continue` and `Break` variants

### Literal Protocols

- [x] `ExpressibleByIntegerLiteral`, `ExpressibleByFloatLiteral`, `ExpressibleByStringLiteral`, `ExpressibleByBoolLiteral`
- [x] `ExpressibleByNilLiteral`, `ExpressibleByArrayLiteral`, `ExpressibleByDictionaryLiteral`
- [x] Default literal type system via `@builtin` annotations
- [x] Array literals with `_ExpressibleByArrayLiteral` protocol

### Pattern Matching & Protocols

- [x] `Matchable` protocol with `matches(self, other: Self) -> Bool`
- [x] `BooleanConditional` protocol for custom boolean conditions
- [x] `Formattable` protocol with `format() -> String` method

### Type System Enhancements

- [x] Init where clauses: `init[T](params) where T: Protocol`
- [x] Associated types in extensions
- [x] Default generic substitution
- [x] Self type improvements in method return types and parameters

### Language Intrinsics

- [x] Cast intrinsics: `lang.cast_<from>_<to>(value)`
- [x] Integer intrinsics: Add, Sub, Mul, Eq, Ne, And, Or, Xor, Shl, Div, Rem, Shr
- [x] Float intrinsics: Add, Sub, Mul, Div, comparisons, Neg, Floor, Ceil, Round, Trunc, Sqrt
- [x] Pointer intrinsics: `ptr.null`, `ptr.read`, `ptr.write`, `sizeof[T]`, `alignof[T]`
- [x] Atomic intrinsics: `atomic.add`, `atomic.sub`
- [x] Builtins system with `lang` namespace

### Syntax Improvements

- [x] Enum cases without labels: `case Some(T)`
- [x] Delegating initializers: `self.init(...)`
- [x] String escape codes: `\n`, `\r`, `\t`, `\xNN`, `\u{NNNN}`, raw strings
- [x] Multi-file spans for accurate error locations
- [x] Optional semicolons in type aliases

### Compiler Infrastructure

- [x] Standard library integration with `--std` and `--no-std` flags
- [x] Optimization levels: `-O` / `--opt-level` (0-2)
- [x] Multi-file compilation
- [x] Aggregate return ABI (SRET)

### Standard Library

- [x] Build standard library
- [x] Build IO
- [x] Build pong
- [x] String escape codes
- [x] Cleanup
- [x] Remove old STD
- [x] Move IO into standard library
- [x] Build with std by default, add --no-std flag
- [x] Fix matches on non-primitive types
- [x] Add Formattable protocol
- [x] Remove deinit + copyable warning
- [x] Error for try
- [x] Fix test suite
- [x] Reference counting
- [x] Add tests for features
- [x] Add tests for STD

## Phase 14 Syntactic Sugar

### Types

- [x] Array Type Syntax
- [x] Dictionary Type Syntax
- [x] Optional Type Syntax
- [x] Result Type Syntax

### Expressions

- [x] Try Operator
- [x] For Loops
- [x] And / Or Short Circuiting
- [x] Null Coalescing Operator
- [ ] Optional Chaining
- [x] Compound Assignment Operators
- [x] Character Literals
- [ ] String Interpolation
- [x] Null Literals
- [ ] Dictionary Literals

### Standard Library

- [x] Create real hash implementation

### Core Features

- [ ] Irrefutable patterns in function parameters
- [ ] Refactor Matchable for range patterns and array patterns

### Goals

- [ ] Web Server
- [ ] Flock package manager
- [ ] Jessup version manager
