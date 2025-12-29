# Kestrel TODO

## Phase 10: Execution Graph

### Completed

- [x] Execution Graph IR
  - [x] Basic block representation (BasicBlock with statements + terminator)
  - [x] Control flow graph structure (FunctionDef with blocks, entry_block)
  - [x] Lower semantic-tree to execution-graph
  - [x] MirContext with arenas for all items
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
  - [x] Call operations (Direct, Thin, Thick, Witness)
  - [x] Struct/tuple/array/enum construction
  - [x] Cast operations, string operations, pointer operations
  - [x] Closure operations (FuncToEscaping, ApplyPartial)
  - [x] IntToString operation
- [x] Item Lowering
  - [x] Functions, initializers, structs, enums, protocols, witnesses, extensions
- [x] Expression Lowering
  - [x] All literals, variable references, field access, assignment
  - [x] All primitive method calls (including Int.toString())
  - [x] Control flow (if/else, if-let, while, while-let, loop, break, continue, return)
  - [x] Match expressions with decision tree compilation
  - [x] Closures (capturing and non-capturing)
  - [x] Method references as values (bound methods)
- [x] Pattern Lowering
  - [x] Irrefutable: local, wildcard, tuple, struct, enum, array prefix, @
  - [x] Refutable: enum variants, literals, ranges
- [x] Pass System
  - [x] MirPass and FunctionPass traits
  - [x] PassManager with sequential execution and fixed-point iteration

### Remaining

- [ ] Analysis Infrastructure
  - [ ] CFG traversal utilities (dominator computation, post-order traversal)
  - [ ] Dataflow analysis framework (reaching definitions, liveness)
  - [ ] Loop detection utilities

- [ ] Optimization Passes
  - [ ] Dead code elimination
  - [ ] Constant folding/propagation
  - [ ] Copy propagation
  - [ ] Inlining

- [ ] Remaining Features
  - [ ] Thin closure optimization (when no captures)
