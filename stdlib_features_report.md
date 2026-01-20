# Kestrel Compiler Features Report
## Changes between `feature/codegen` and `887799a` (fix: fixed formatting)

This report synthesizes the semantic and feature changes across 53 commits.

---

## Major Language Features Added

### 1. Computed Properties
**Commits:** `16408ac`, `6a7d479`, `c84b102`

Full computed property support with getter/setter accessors:
- Shorthand syntax: `var x: Int { expr }`
- Explicit accessors: `var x: Int { get { expr } set { expr } }`
- Protocol requirements: `{ get }` or `{ get set }`
- New `GetterSymbol` and `SetterSymbol` types
- Computed properties excluded from struct cycle detection
- Protocol conformance checking for property requirements

### 2. Subscripts
**Commit:** `826765a`, `b0ce6dc`

Array/dictionary-style subscript access:
- Syntax: `subscript[T]?(params) -> Type { body }`
- Three body forms: shorthand, explicit get/set, protocol requirements
- `ExprKind::SubscriptCall` for `receiver[args]` expressions
- Full parser, binder, and lowerer pipeline
- Extension and protocol support for subscripts
- First-class function calls on fields with function types

### 3. Protocol Extensions & Default Implementations
**Commit:** `89914de`

Protocol-level method extensions:
- `extend Protocol { ... }` syntax for adding default implementations
- New `Constraint::SelfBound` for conditional extensions: `extend Proto where Self: OtherProto`
- Associated type constraints in extensions
- Methods automatically available to all conforming types
- Specificity-based extension resolution

### 4. Protocol Operators
**Commit:** `8ed3b49`

Operator overloading via protocol conformance:
- 58 new operator protocols: `AddOperatorProtocol`, `SubtractOperatorProtocol`, etc.
- Operators desugar to protocol method calls
- Enables custom types to implement operators
- Removed identity operator (`+x`)
- Standardized operator method naming

### 5. Try Operator
**Commit:** `222f894`

Rust-style error handling:
- `try expr` syntax with high precedence
- Desugars to match on `tryExtract()` method
- `Tryable` and `FromResidual` protocols
- `ControlFlowEnum` with `Continue` and `Break` variants

### 6. Literal Protocols
**Commit:** `4ac0425`

Custom literal handling:
- `ExpressibleByIntegerLiteral`, `ExpressibleByFloatLiteral`, `ExpressibleByStringLiteral`, `ExpressibleByBoolLiteral`
- `ExpressibleByNilLiteral`, `ExpressibleByArrayLiteral`, `ExpressibleByDictionaryLiteral`
- Default literal type system via `@builtin` annotations
- Literals resolve to inference types, then conform via protocols

### 7. Array Literals
**Commit:** `264fdb8`

Stack-based array literal implementation:
- `Rvalue::StackAlloc` replaces primitive array construction
- Arrays lowered to stack allocation + initializer calls
- `_ExpressibleByArrayLiteral` protocol with `init(_arrayLiteralPointer:_arrayLiteralCount:)`
- Custom allocator support

### 8. Matchable Protocol
**Commit:** `24536af`

Custom pattern matching:
- `Matchable` protocol with `matches(self, other: Self) -> Bool`
- Types can define custom equality for pattern matching
- Literal patterns use witness method calls
- Fallback to direct comparison for non-Matchable types

### 9. Formattable Protocol
**Commit:** `130dc3a`

Universal string formatting:
- `Formattable` protocol with `format() -> String` method
- All numeric types implement Formattable
- `print()`, `println()` accept any `Formattable` type
- Float formatting with 6 decimal precision

---

## Type System Enhancements

### 10. Type Where Clauses & Init Where Clauses
**Commit:** `9214ae2`

Extended constraint syntax:
- Initializer type parameters: `init[T](params) where T: Protocol`
- Type alias where clauses: `type Alias where Iter.Item = Item`
- Removed unsigned integer types from `LangPrimitive` (2's complement model)
- Added `I1` type for booleans

### 11. Associated Types in Extensions
**Commit:** `f4712e8`

- Extensions can now declare associated types
- `TypeAlias` variant added to `ExtensionBodyItem`
- Primitive type implicit protocol conformances

### 12. Default Generic Substitution
**Commit:** `8fabef6`

- `apply_type_arguments_if_all_have_defaults()` for auto-applying defaults
- Recursive associated type substitution with depth limit
- Type parameter default resolution at parse time

### 13. Self Type Improvements
**Commit:** `8443be6`, `79b6639`

- Proper `Self` substitution in method return types and parameters
- `ContextualOracle` for function-aware type resolution
- Protocol inheritance traversal for bound checking
- Concrete self type resolution in struct/enum methods

---

## Language Intrinsics

### 14. Cast Intrinsics
**Commit:** `f498634`

- `LangPrimitive` enum for primitive types
- `LangIntrinsic` for compiler-provided functions
- `lang.panic_unwind(message)` for diverging panic
- `lang.cast_<from>_<to>(value)` for explicit type casts

### 15. Integer & Float Intrinsics
**Commit:** `9214ae2`

Comprehensive low-level operations:
- `IntBinary`: Add, Sub, Mul, Eq, Ne, And, Or, Xor, Shl
- `IntBinarySigned`/`IntBinaryUnsigned`: Div, Rem, Shr, comparisons
- `FloatBinary`: Add, Sub, Mul, Div, comparisons
- `FloatUnary`: Neg
- `FloatConst`: Infinity, Nan
- `FloatPred`: IsNan, IsInfinite
- `FloatMath`: Floor, Ceil, Round, Trunc, Sqrt

### 16. Pointer & Atomic Intrinsics
**Commit:** `176c468`

Low-level memory operations:
- `ptr.null`, `ptr.from_address`, `ptr.to_address`
- `ptr.read`, `ptr.write`, `ptr.is_null`, `ptr.cast`
- `sizeof[T]`, `alignof[T]`
- `i1.eq`, `i1.and`, `i1.or`, `i1.not`
- `atomic.add`, `atomic.sub`

### 17. Builtins System
**Commit:** `86f556f`

- `lang` namespace for compiler-intrinsic types
- Primitives: `lang.i1`, `lang.i8`-`lang.i64`, `lang.f16`-`lang.f64`, `lang.str`
- F16 support added (partial implementation)
- `FFISafe` automatic conformance for primitives

---

## Syntax & Parser Features

### 18. Enum Cases Without Labels
**Commit:** `6d5f1be`

- Unnamed enum parameters: `case Some(T)` vs `case Some(value: T)`
- Synthetic parameter names generated (`_0`, `_1`, etc.)

### 19. Delegating Initializers
**Commit:** `6d5f1be`

- `self.init(...)` for initializer delegation
- Validates delegation within initializer context
- All fields marked initialized after delegation

### 20. String Escape Codes
**Commit:** `5af8877`

Comprehensive string escape support:
- Standard escapes: `\n`, `\r`, `\t`, `\\`, `\"`, `\'`, `\0`
- Hex ASCII: `\xNN` (0x00-0x7F)
- Unicode: `\u{NNNN}` (1-6 hex digits)
- Line continuation: backslash + newline
- Raw strings: `"""content"""`

### 21. Multi-file Spans
**Commit:** `7e2a1ba`

- `file_id` tracking in `EventSink`
- Accurate error locations across multiple source files

### 22. Optional Semicolons in Type Aliases
**Commit:** `49c00f0`

- Semicolon made optional in type alias declarations
- Supports protocol-associated types without trailing semicolon

---

## Compiler Infrastructure

### 23. Standard Library Integration
**Commit:** `77d375b`

- Automatic stdlib loading with `--std` and `--no-std` flags
- `StdLibConfig` with multi-location path resolution
- Synthetic wildcard imports for `std.*` modules
- `Span::synthetic()` for compiler-generated code

### 24. Optimization Levels
**Commit:** `966f0b1`

- `-O` / `--opt-level` CLI flag (0-2)
- Propagated through compilation pipeline to Cranelift

### 25. Multi-file Compilation
**Commit:** `7ad263a`

- `Build` and `Run` commands accept multiple files
- Files combined into single compilation unit

---

## Codegen & ABI Improvements

### 26. Aggregate Return ABI (SRET)
**Commit:** `ea508f1`

- Struct return type calling convention
- Implicit pointer parameter for aggregate returns
- Stack allocation for aggregate locals
- Proper aggregate value copying

### 27. Wrapper Type Unwrapping
**Commit:** `0dab197`, `d634a82`

- Automatic unwrapping of single-field wrapper types for extern calls
- `wrap_extern_return_value()` for C ABI compatibility
- Proper boolean condition handling in branches

### 28. Protocol Extension Method Codegen
**Commit:** `40f4098`, `6f45207`

- Self type tracking in `FunctionInstantiation`
- Witness dispatch for protocol methods
- Associated type resolution in method signatures
- Getter/setter lowering for computed properties

### 29. Enum Witness Generation
**Commit:** `7e27f86`

- Witness generation for enum protocol conformances
- Derived witness generation from protocol extensions
- Method-specific type argument tracking

---

## Bug Fixes & Improvements

### 30. Cross-module Protocol Conformances
**Commit:** `7c92afa`

- Import resolution in scope for protocol lookup
- `visible_children()` for cross-file declarations

### 31. Enum Exhaustivity
**Commit:** `ef03f1d`

- Proper `Self` type resolution in pattern matching
- `resolve_match_scrutinee_type()` for extensions

### 32. Duplicate Detection
**Commit:** `d373a5e`, `c7818df`

- Label-based overloading model with `DuplicateKey`
- Protocol conformance signature tracking
- Extension initializers support

### 33. BooleanConditional Protocol
**Commit:** `64af273`

- Conditions check `BooleanConditional` conformance
- Custom types can act as boolean conditions

### 34. Default RHS for Operators
**Commit:** `29bd3da`

- Parameter type capture in `MemberResolution`
- Argument-parameter type constraints for literal inference

### 35. Type Resolution Fixes
**Commit:** `aa9104a`

- Error type propagation
- Duplicate/shadowing type parameter detection
- SelfType resolution before member access

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Major Language Features | 9 |
| Type System Enhancements | 4 |
| Language Intrinsics | 4 |
| Syntax & Parser Features | 5 |
| Compiler Infrastructure | 3 |
| Codegen & ABI | 4 |
| Bug Fixes | 6 |
| **Total Significant Changes** | **35** |

---

*Generated from analysis of 53 commits between `feature/codegen` and `887799a`*
