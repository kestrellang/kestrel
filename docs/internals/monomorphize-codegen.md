# Monomorphization During Codegen

This document describes the design and implementation plan for monomorphizing generic functions, structs, and enums during the codegen phase.

## Overview

Kestrel uses monomorphization to implement generics - each unique instantiation of a generic item (e.g., `identity[Int]`, `identity[Bool]`) becomes a separate compiled entity. Rather than transforming the MIR, monomorphization happens "on the fly" during codegen:

```
MirContext (generic, mostly unchanged)
    │
    ▼ collect_all()
MonomorphizationSet { functions, structs, enums }
    │
    ▼ for each instantiation: compile with substitution
Cranelift IR (monomorphized)
```

The MIR remains generic. When compiling a function instantiation like `identity[Int]`, we carry a `Substitution` through codegen and apply it as we translate each statement.

## Key Concepts

### Instantiation

An instantiation is a generic item applied to concrete type arguments:

- `identity[Int]` - function instantiation
- `Box[Int]` - struct instantiation  
- `Option[String]` - enum instantiation

### Substitution

A mapping from type parameters to concrete types. For `func identity[T](x: T) -> T` instantiated as `identity[Int]`, the substitution is `{T → Int}`.

### Witness Resolution

Protocol method calls on type parameters (e.g., `x.clone()` where `x: T` and `T: Cloneable`) are represented as `Callee::Witness` in MIR. During monomorphization, these are resolved to direct calls by looking up the witness table.

## Collection Phase

### Algorithm

The collection phase uses BFS to discover all instantiations:

```
1. Build indices:
   - functions_by_name: Map<QualifiedName, Function>
   - structs_by_name: Map<QualifiedName, Struct>
   - enums_by_name: Map<QualifiedName, Enum>

2. Seed work queue:
   - All non-generic functions (with empty substitution)
   - All statics (scan their types)

3. BFS loop:
   while pending_functions not empty:
     - Pop (func_id, subst)
     - Scan function with substitution:
       - Param/return/local types → scan_type
       - Statements/terminators:
         - Callee::Direct with type_args → apply subst, record FunctionInstantiation
         - Callee::Witness → resolve witness, record FunctionInstantiation
         - ImmediateKind::FunctionRef → same as Direct
         - ImmediateKind::WitnessMethod → same as Witness
         - All types encountered → scan_type

4. scan_type(ty, subst):
   - Apply subst to ty (may intern new types)
   - If Named with non-empty type_args:
     - Record StructInstantiation or EnumInstantiation
   - Recurse into nested types
```

### Nested Generics

When a generic function calls another generic function with its type parameter:

```kestrel
func process[U](x: U) -> U {
    let boxed = Box(x)      // Box[U]
    unwrap(boxed)           // unwrap[U]
}

func main() {
    process(42)  // process[Int]
}
```

Collection trace:
1. Seed: `main` (non-generic)
2. Scan `main`: find `process[Int]` → queue `FunctionInstantiation { func: process, type_args: [Int] }`
3. Process `process[Int]` with subst `{U → Int}`:
   - `Box[U]` → `subst.apply_ty` → `Box[Int]` → record `StructInstantiation`
   - `unwrap[U]` → `subst.apply_ty(U)` → `Int` → record `FunctionInstantiation { func: unwrap, type_args: [Int] }`
4. Process `unwrap[Int]` with subst `{T → Int}`:
   - Scan body, no more instantiations

The substitution mechanism handles nesting naturally.

### Type Interning

`apply_ty` may create new composite types (e.g., `Box[Int]` from `Box[T]`). These are interned into `MirContext` during collection, which requires `&mut MirContext`.

## Witness Resolution

### Witness Structure

A witness proves that a type implements a protocol:

```
witness Box[T]: Cloneable where T: Cloneable {
    type_params: [T]
    func clone = Box.clone
}
```

### Resolution Algorithm

Given `Callee::Witness { protocol, method, for_type }`:

```
1. Apply current substitution to for_type → concrete_type

2. Find matching witness:
   - Iterate witnesses where witness.protocol == protocol
   - Try to match witness.implementing_type against concrete_type
   
3. Extract type arguments:
   - Pattern match witness.implementing_type (e.g., Box[T]) against concrete_type (e.g., Box[Int])
   - Extract bindings: {T → Int}
   
4. Look up method binding:
   - witness.method_bindings[method] → implementation function name
   
5. Return (impl_func_id, extracted_type_args)
```

### Pattern Matching for Witnesses

Matching `Box[Int]` against `Box[T]`:

```rust
fn match_pattern(pattern: Id<Ty>, concrete: Id<Ty>, mapping: &mut Map<TypeParam, Ty>) {
    match (pattern, concrete) {
        // TypeParam: bind it
        (TypeParam(tp), _) => {
            mapping.insert(tp, concrete);
        }
        
        // Named: match name, recurse into type_args
        (Named { name: n1, type_args: args1 }, Named { name: n2, type_args: args2 }) 
            if n1 == n2 && args1.len() == args2.len() => {
            for (a1, a2) in args1.zip(args2) {
                match_pattern(a1, a2, mapping);
            }
        }
        
        // Structural types: recurse
        (Ref(a), Ref(b)) => match_pattern(a, b, mapping),
        // ... etc
        
        // Primitives: must be equal
        (a, b) if a == b => Ok(()),
        
        _ => Err(TypeMismatch)
    }
}
```

This is simple structural matching, not full unification.

### Witness Call Example

```kestrel
struct Box[T] { value: T }

extend[T: Cloneable] Box[T]: Cloneable {
    func clone(ref self) -> Box[T] {
        Box(self.value.clone())  // Witness call to T.clone()
    }
}

func main() {
    let b = Box(42)
    let c = b.clone()  // Witness call: Cloneable.clone for Box[Int]
}
```

Resolution for `b.clone()`:
1. `Callee::Witness { protocol: Cloneable, method: "clone", for_type: Box[Int] }`
2. Find witness `Box[T]: Cloneable`
3. Match `Box[T]` against `Box[Int]` → `{T → Int}`
4. Look up `method_bindings["clone"]` → `Box.clone`
5. Result: `Callee::Direct { name: Box.clone, type_args: [Int] }`

Inside `Box.clone[Int]`, there's another witness call `self.value.clone()`:
1. `Callee::Witness { protocol: Cloneable, method: "clone", for_type: T }`
2. With subst `{T → Int}`, `for_type` becomes `Int`
3. Find witness `Int: Cloneable`
4. Result: `Callee::Direct { name: Int.clone, type_args: [] }`

## Codegen Phase

### Function Declaration

Declare both non-generic functions and instantiated generic functions:

```rust
fn declare_all_functions(&mut self) {
    // Non-generic functions
    for (func_id, func_def) in mir.functions.iter() {
        if func_def.type_params.is_empty() {
            declare_function(func_def, &[]);
        }
    }
    
    // Generic instantiations
    for inst in &mono_set.functions {
        let func_def = &mir.functions[inst.func_id];
        declare_function(func_def, &inst.type_args);
    }
}
```

### Name Mangling

Generic functions are mangled with their type arguments:

| Call | Mangled Name |
|------|--------------|
| `identity[Int]` | `_K8identityIiE` |
| `Box[Int].clone` | `_K3BoxIiE5clone` |
| `process[Box[Int]]` | `_K7processI3BoxIiEE` |

### Threading Substitution

The `Substitution` is passed through all codegen functions:

```rust
fn compile_function_body(ctx, func_def, subst: &Substitution, ...) {
    // ...
}

fn compile_block(ctx, block, subst: &Substitution, ...) {
    // ...
}

fn compile_rvalue(ctx, rvalue, subst: &Substitution, ...) {
    match rvalue {
        Rvalue::Call { callee, .. } => {
            match callee {
                Callee::Direct { name, type_args } => {
                    // Apply substitution to type_args
                    let concrete_args = type_args.iter()
                        .map(|ty| subst.apply_ty_readonly(mir, ty))
                        .collect();
                    let mangled = mangle_name(name, &concrete_args);
                    // ... emit call
                }
                
                Callee::Witness { protocol, method, for_type } => {
                    let concrete_for_type = subst.apply_ty_readonly(mir, for_type);
                    let (impl_func, impl_type_args) = resolve_witness(
                        protocol, method, concrete_for_type
                    );
                    let mangled = mangle_name(impl_func, &impl_type_args);
                    // ... emit call
                }
            }
        }
    }
}
```

### Read-Only Type Application

During codegen, `MirContext` is read-only. All types needed should have been interned during collection. `apply_ty_readonly` looks up types without interning:

```rust
fn apply_ty_readonly(&self, mir: &MirContext, ty: Id<Ty>) -> Id<Ty> {
    match mir.ty(ty) {
        MirTy::TypeParam(tp) => self.mapping.get(tp).unwrap_or(ty),
        
        MirTy::Named { name, type_args } => {
            let new_args = type_args.iter()
                .map(|arg| self.apply_ty_readonly(mir, arg))
                .collect();
            if new_args == type_args {
                ty
            } else {
                mir.lookup_type(&MirTy::Named { name, type_args: new_args })
                    .expect("type should have been interned during collection")
            }
        }
        
        // ... other cases
    }
}
```

## Data Structures

### Instantiation Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionInstantiation {
    pub func_id: Id<Function>,
    pub type_args: Vec<Id<Ty>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructInstantiation {
    pub struct_id: Id<Struct>,
    pub type_args: Vec<Id<Ty>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumInstantiation {
    pub enum_id: Id<Enum>,
    pub type_args: Vec<Id<Ty>>,
}

#[derive(Debug, Default)]
pub struct MonomorphizationSet {
    pub functions: HashSet<FunctionInstantiation>,
    pub structs: HashSet<StructInstantiation>,
    pub enums: HashSet<EnumInstantiation>,
}
```

### Substitution

```rust
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    mapping: HashMap<Id<TypeParam>, Id<Ty>>,
}

impl Substitution {
    pub fn new() -> Self;
    pub fn is_empty(&self) -> bool;
    pub fn insert(&mut self, tp: Id<TypeParam>, ty: Id<Ty>);
    pub fn get(&self, tp: Id<TypeParam>) -> Option<Id<Ty>>;
    
    /// Apply substitution, interning new types as needed (collection phase)
    pub fn apply_ty(&self, mir: &mut MirContext, ty: Id<Ty>) -> Id<Ty>;
    
    /// Apply substitution, looking up types without interning (codegen phase)
    pub fn apply_ty_readonly(&self, mir: &MirContext, ty: Id<Ty>) -> Id<Ty>;
}

pub fn build_substitution(
    mir: &MirContext,
    type_params: &[Id<TypeParam>],
    type_args: &[Id<Ty>],
) -> Substitution;
```

### Errors

```rust
pub enum MonomorphizeError {
    WitnessNotFound {
        protocol: Id<QualifiedName>,
        for_type: Id<Ty>,
    },
    MethodNotFoundInWitness {
        protocol: Id<QualifiedName>,
        method: String,
        for_type: Id<Ty>,
    },
    FunctionNotFound {
        name: Id<QualifiedName>,
    },
    TypeMismatch {
        expected: Id<Ty>,
        found: Id<Ty>,
    },
}
```

Errors are accumulated during collection and reported together.

## File Structure

```
lib/kestrel-codegen-cranelift/src/
├── monomorphize/
│   ├── mod.rs              // Public API, MonomorphizationSet
│   ├── instantiation.rs    // FunctionInstantiation, StructInstantiation, EnumInstantiation
│   ├── error.rs            // MonomorphizeError
│   ├── substitute.rs       // Substitution, apply_ty, build_substitution
│   ├── witness.rs          // find_witness, match_pattern, resolve_witness
│   └── collect.rs          // CollectionContext, collect_all (BFS)
├── context.rs              // Modified: store MonomorphizationSet
├── function.rs             // Modified: thread Substitution
├── rvalue.rs               // Modified: resolve Callee::Witness
├── block.rs                // Modified: thread Substitution
├── terminator.rs           // Modified: thread Substitution (if needed)
├── error.rs                // Modified: add Monomorphization variant
└── lib.rs                  // Modified: add module, update compile signature
```

Also modify:
```
lib/kestrel-execution-graph/src/
└── lib.rs                  // Add lookup_type method to MirContext
```

## API Changes

### compile Function

```rust
// Before
pub fn compile(
    mir: &MirContext,
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError>

// After
pub fn compile(
    mir: &mut MirContext,  // Mutable for type interning during collection
    target: &TargetConfig,
    options: &CodegenOptions,
) -> Result<CompilationResult, CodegenError>
```

### MirContext

```rust
impl MirContext {
    /// Look up an already-interned type by its structure.
    pub fn lookup_type(&self, ty: &MirTy) -> Option<Id<Ty>> {
        self.type_lookup.get(ty).copied()
    }
}
```

## Implementation Order

1. `kestrel-execution-graph/src/lib.rs` - Add `lookup_type`
2. `monomorphize/error.rs` - Error types
3. `monomorphize/instantiation.rs` - Instantiation data types
4. `monomorphize/substitute.rs` - Substitution with `apply_ty` and `apply_ty_readonly`
5. `monomorphize/witness.rs` - Witness lookup and resolution
6. `monomorphize/collect.rs` - BFS collection algorithm
7. `monomorphize/mod.rs` - Wire together, exports
8. `error.rs` - Add `Monomorphization` variant to `CodegenError`
9. `lib.rs` - Add module, update `compile` signature
10. `context.rs` - Store `MonomorphizationSet`, update declare/define
11. `function.rs` - Thread `Substitution` through
12. `rvalue.rs` - Handle `Callee::Witness` and `ImmediateKind::WitnessMethod`
13. `block.rs`/`terminator.rs` - Thread substitution if needed
14. Unit tests (in `monomorphize/*.rs`)
15. Integration tests (in `kestrel-test-suite/tests/codegen/`)

## Test Cases

### Unit Tests

**substitute.rs:**
- `apply_ty` on primitive types (unchanged)
- `apply_ty` on `TypeParam` (substituted)
- `apply_ty` on `Named` with type params (recursive)
- `apply_ty` on nested types like `Ref(Named(...))`
- `build_substitution` from type params and args

**witness.rs:**
- `match_pattern` on exact match
- `match_pattern` on `TypeParam` binding
- `match_pattern` on nested `Named` types
- `find_witness` with non-generic witness
- `find_witness` with generic witness
- `resolve_witness` end-to-end

**collect.rs:**
- Collection from non-generic function calling generic
- Transitive collection (generic calls generic)
- Witness call collection
- Struct/enum instantiation collection
- Nested generics like `Box[Box[Int]]`

### Integration Tests

Full pipeline tests (parse → semantic → lower → collect → codegen):

1. Basic generic function: `identity[T](x: T) -> T` called with `Int`
2. Nested generic calls: `foo[T]` calls `bar[T]`, instantiate `foo[Int]`
3. Generic struct: `Box[T]` instantiated as `Box[Int]`
4. Generic enum: `Option[T]` instantiated as `Option[Int]`
5. Nested type args: `Box[Box[Int]]`
6. Simple witness: `Point: Cloneable` (non-generic)
7. Generic witness: `Box[T]: Cloneable where T: Cloneable`
8. Witness chain: `clone` on `Box[Int]` calls `clone` on `Int`
9. Multiple instantiations: Same generic with different type args
10. Function reference: `ImmediateKind::FunctionRef` with type args
11. Witness method value: `ImmediateKind::WitnessMethod`

## Future Considerations

### Existentials

If we add existentials (like `dyn Protocol`), witness calls would NOT be monomorphized - they'd use runtime vtable dispatch. The `Callee::Witness` would remain in codegen and emit vtable lookup code instead of being resolved to a direct call.

### Optimization

Current witness lookup is linear scan. If this becomes a bottleneck, add indices:
- `HashMap<Id<QualifiedName>, Vec<Id<Witness>>>` - witnesses by protocol
- `HashMap<(Id<QualifiedName>, Id<QualifiedName>), Id<Witness>>` - by (protocol, type name)

### Incremental Compilation

The current design recomputes all instantiations from scratch. For incremental compilation, we'd need to:
- Cache the `MonomorphizationSet`
- Track dependencies between functions and instantiations
- Invalidate/recompute when source changes
