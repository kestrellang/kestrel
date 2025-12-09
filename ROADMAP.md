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
- [ ] Protocol Method Linking
  - [ ] Track which protocol a method satisfies when struct conforms
  - [ ] Resolve protocol method calls to concrete implementations
  - [ ] ProtocolImplementationBehavior for method bindings
- [ ] Extensions with Conformances
  - [ ] `extend Type: Protocol { ... }` syntax
  - [ ] ExtensionSymbol and extension registry
  - [ ] Methods in extension satisfy protocol requirements
  - [ ] Retroactive conformance (add conformance to types you don't own)

### Deferred to Later

- [ ] Static Methods on Type Parameters (`T.staticMethod()`)
- [ ] Tighter Type Parameter Assignability

## Phase 7: Type Inference

- [ ] Local Type Inference
  - [ ] `let x = 42` infers `Int`
  - [ ] `let p = Point(x: 1, y: 2)` infers `Point`
- [ ] Generic Type Argument Inference
  - [ ] Infer type arguments from call arguments
  - [ ] `identity(42)` infers `identity[Int](42)`
- [ ] Bidirectional Type Checking
  - [ ] Expected type propagation into expressions
  - [ ] Foundation for closure parameter inference

## Phase 8: Closures & First-Class Functions

- [ ] Closure Expressions
  - [ ] Closure syntax (e.g., `{ x, y in x + y }` or `func(x, y) { x + y }`)
  - [ ] Capturing variables from enclosing scope
  - [ ] Capture semantics (by value vs by reference)
- [ ] Function References
  - [ ] Reference named functions as values
  - [ ] Pass functions to higher-order functions
- [ ] Closure Type Inference
  - [ ] Infer parameter types from context
  - [ ] `numbers.map({ n in n * 2 })` infers `n: Int`

## Phase 9: Enums & Algebraic Data Types

- [ ] Enum Declarations
  - [ ] Simple enums: `enum Color { Red, Green, Blue }`
  - [ ] Enums with associated values: `enum Option[T] { Some(T), None }`
  - [ ] Recursive enums
- [ ] Pattern Matching
  - [ ] `match` expressions
  - [ ] Exhaustiveness checking
  - [ ] Patterns: literals, bindings, enum variants, wildcards
  - [ ] Guard clauses in patterns
  - [ ] `if let` / `guard let`

## Phase 10: Memory Model

- [ ] Value vs Reference Semantics
  - [ ] Structs as value types (copy semantics)
  - [ ] Reference types if needed
- [ ] Ownership Strategy
  - [ ] Reference counting, or
  - [ ] Ownership/borrowing, or
  - [ ] Garbage collection

## Phase 11: Code Generation

- [ ] IR Generation
  - [ ] Choose target: LLVM, WASM, bytecode, or transpile
- [ ] Runtime Support
  - [ ] Memory management implementation
  - [ ] Built-in function implementations
- [ ] Executable Output
  - [ ] Binary or interpreted execution

## Phase 12: Standard Library & Syntactic Sugar

- [ ] Standard Library
  - [ ] Option[T], Result[T, E] (as regular enums)
  - [ ] Collections (Array, Map, Set)
  - [ ] String utilities
  - [ ] I/O primitives
- [ ] Syntactic Sugar
  - [ ] `T?` for `Option[T]`
  - [ ] `?` operator for error/option propagation
  - [ ] Optional chaining `x?.foo`
  - [ ] For loops (desugars to iterator protocol)

---

## Current Status

**Phase**: Phase 6 (Generics & Protocols) - IN PROGRESS
**Progress**: Phases 1-5 complete. Phase 6 ~75% complete (3 of 4 major features done).

**Recently Completed (Phase 6)**:

- Generic Constraint Enforcement ✓
  - Method calls on type parameters via protocol bounds (`T.method()` where `T: Protocol`)
  - Self substitution in protocol method return types and parameters
  - Ambiguous method detection across multiple bounds
  - Protocol inheritance chain traversal for method lookup
  - Call-site constraint verification
  - New diagnostics: UnconstrainedTypeParameterMemberError, MethodNotInBoundsError, AmbiguousConstrainedMethodError, ConstraintNotSatisfiedError, UnsupportedGenericProtocolBoundError
- GenericsBehavior Refactor ✓
  - Replaced RwLock<WhereClause> with clean GenericsBehavior pattern
  - Type parameters and where clause now stored as behavior (like CallableBehavior)
  - Resolvers add GenericsBehavior during BIND with fully resolved protocol bounds
  - Fallback to children for type_parameters() during BUILD phase
- Associated Types ✓
  - Protocol associated type declarations (`type Item` in protocols)
  - AssociatedTypeSymbol representation with constraints and defaults
  - Qualified type path resolution (`T.Item`, `C.Iter.Item`)
  - Associated type bindings in conforming structs (`type Item = Int`)
  - Qualified bindings for disambiguation (`type Iterator.Item = Int`)
  - Protocol inheritance with where clause constraints (`where Iterator.Item: Comparable`)
  - Constraint satisfaction validation for bindings
  - New diagnostics: AmbiguousAssociatedTypeError, QualifiedBindingNotConformingError, QualifiedBindingWrongProtocolError, WhereClauseAssociatedTypeNotFoundError, AssociatedTypeConstraintNotSatisfiedError

**Next Tasks**:

1. Protocol method linking (Phase 6)
2. Extensions with conformances (Phase 6)

## Notes

- Structs replace classes for a simpler, more flexible type system
- Protocols provide interface abstraction without inheritance complexity
- Functions are first-class, enabling functional programming patterns
- Function overloading supported via arity, types, and labels
- Labeled parameters enable Swift-style named arguments
- Standard library and syntactic sugar come last - core language first
