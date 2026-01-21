# Type Inference Design: `kestrel-semantic-type-inference`

## Overview

A Hindley-Milner style constraint-based type inference system for Kestrel. The solver collects constraints, then iterates to a fixpoint, resolving types and member accesses along the way.

## Key Design Decisions

| Decision | Choice |
|----------|--------|
| Type representation | Reuse `Ty` directly |
| Inference marker | `TyKind::Infer` (no payload) |
| ID assignment | Every `Ty::new()` and `Expression::new()` gets fresh ID |
| Constraints | `Equals`, `Conforms`, `Normalizes`, `MemberAccess` |
| Resolution | TypeOracle trait (callback-based) |
| Solving | One-pass fixpoint iteration |
| Solution | `HashMap<TyId, Ty>` + `HashMap<ExprId, ValueResolution>` |

## Crate Structure

```
lib/kestrel-semantic-type-inference/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── constraint.rs      # Constraint enum
    ├── context.rs         # InferenceContext
    ├── oracle.rs          # TypeOracle trait
    ├── solver.rs          # Unification + fixpoint loop
    ├── solution.rs        # Solution type
    └── error.rs           # InferenceError
```

## Dependencies

```toml
[dependencies]
kestrel-semantic-tree = { path = "../kestrel-semantic-tree" }  # For Ty, TyKind, etc.
```

Should NOT depend on:
- `kestrel-semantic-model` (oracle is injected)
- `kestrel-semantic-tree-binder` (constraint generation happens elsewhere)

## Core Types

### Constraints

```rust
// constraint.rs
pub enum Constraint {
    /// Two types must be equal: τ₁ = τ₂
    Equals(TyId, TyId),

    /// A type must conform to a protocol: τ : Protocol
    Conforms(TyId, ProtocolRef),

    /// Associated type projection: T.Assoc => τ
    Normalizes {
        base: TyId,
        assoc_name: Name,
        result: TyId,
    },

    /// Member access: receiver.member has type τ
    MemberAccess {
        receiver: TyId,
        member: Name,
        is_static: bool,
        result: TyId,
        expr_id: ExprId,
    },
}
```

### TypeOracle Trait

```rust
// oracle.rs
pub trait TypeOracle {
    /// Look up a member on a type (instance or static).
    fn resolve_member(
        &self,
        receiver_ty: &Ty,
        member: &Name,
        is_static: bool,
    ) -> Result<MemberResolution, MemberError>;

    /// Check if a type conforms to a protocol.
    fn conforms_to(&self, ty: &Ty, protocol: &Ty) -> bool;

    /// Resolve an associated type on a type/protocol.
    fn resolve_associated_type(
        &self,
        container: &Ty,
        assoc_name: &Name,
    ) -> Option<Ty>;
}

pub struct MemberResolution {
    pub ty: Ty,
    pub symbol: SymbolId,
    pub substitutions: Substitutions,
}
```

### Solution

```rust
// solution.rs
pub struct Solution {
    /// Resolved types for all type expression slots
    pub types: HashMap<TyId, Ty>,

    /// Resolved symbols for value expressions that needed type-directed resolution
    pub values: HashMap<ExprId, ValueResolution>,
}

pub struct ValueResolution {
    pub symbol: SymbolId,
    pub substitutions: Substitutions,
}
```

### Errors

```rust
// error.rs
pub enum InferenceError {
    /// Types couldn't be unified: expected τ₁, found τ₂
    TypeMismatch {
        expected: Ty,
        found: Ty,
        span: Span,
    },

    /// Infinite type: T = List[T]
    OccursCheck {
        var: TyId,
        ty: Ty,
    },

    /// Type doesn't conform to protocol
    ConformanceFailure {
        ty: Ty,
        protocol: Ty,
        span: Span,
    },

    /// Member not found on type
    MemberNotFound {
        receiver: Ty,
        member: Name,
        span: Span,
    },

    /// Associated type not found
    AssociatedTypeNotFound {
        container: Ty,
        assoc_name: Name,
        span: Span,
    },

    /// Couldn't fully resolve all types
    Ambiguous {
        unresolved: Vec<TyId>,
    },
}
```

### InferenceContext

```rust
// context.rs
pub struct InferenceContext<'a> {
    oracle: &'a dyn TypeOracle,
    constraints: Vec<Constraint>,
    substitutions: HashMap<TyId, Ty>,
    values: HashMap<ExprId, ValueResolution>,
}

impl<'a> InferenceContext<'a> {
    pub fn new(oracle: &'a dyn TypeOracle) -> Self;

    /// Add equality constraint
    pub fn equate(&mut self, a: TyId, b: TyId);

    /// Add conformance constraint
    pub fn conforms(&mut self, ty: TyId, protocol: ProtocolRef);

    /// Add associated type normalization constraint
    pub fn normalizes(&mut self, base: TyId, assoc: Name, result: TyId);

    /// Add member access constraint
    pub fn member_access(
        &mut self,
        receiver: TyId,
        member: Name,
        is_static: bool,
        result: TyId,
        expr_id: ExprId,
    );

    /// Solve all constraints
    pub fn solve(self) -> Result<Solution, InferenceError>;
}
```

## Solver Algorithm

```rust
// solver.rs (internal)
impl<'a> InferenceContext<'a> {
    pub fn solve(mut self) -> Result<Solution, InferenceError> {
        // Iterate until fixpoint
        loop {
            let progress = self.solve_round()?;
            if !progress {
                break;
            }
        }

        // Ensure everything resolved
        self.check_fully_resolved()?;

        Ok(Solution {
            types: self.substitutions,
            values: self.values,
        })
    }

    fn solve_round(&mut self) -> Result<bool, InferenceError> {
        let mut progress = false;
        let constraints = std::mem::take(&mut self.constraints);

        for constraint in constraints {
            match self.try_solve(&constraint)? {
                SolveResult::Solved => progress = true,
                SolveResult::Deferred => self.constraints.push(constraint),
            }
        }

        Ok(progress)
    }

    fn try_solve(&mut self, c: &Constraint) -> Result<SolveResult, InferenceError> {
        match c {
            Constraint::Equals(a, b) => self.unify(*a, *b),
            Constraint::Conforms(ty, proto) => self.check_conforms(*ty, proto),
            Constraint::Normalizes { base, assoc_name, result } => {
                self.normalize(*base, assoc_name, *result)
            }
            Constraint::MemberAccess { receiver, member, is_static, result, expr_id } => {
                self.resolve_member(*receiver, member, *is_static, *result, *expr_id)
            }
        }
    }

    fn unify(&mut self, a: TyId, b: TyId) -> Result<SolveResult, InferenceError> {
        let ty_a = self.resolve(a);
        let ty_b = self.resolve(b);

        // Standard HM unification with occurs check
        // ... (see implementation)
    }

    fn resolve(&self, id: TyId) -> &Ty {
        // Follow substitution chain
    }

    fn occurs_check(&self, var: TyId, ty: &Ty) -> bool {
        // Check if var occurs in ty (prevents infinite types)
    }
}

enum SolveResult {
    Solved,
    Deferred,
}
```

## Changes to `kestrel-semantic-tree`

### 1. Add `TyId` to `Ty`

```rust
// ty/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TyId(u64);

impl TyId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        TyId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Ty {
    pub id: TyId,       // NEW
    pub kind: TyKind,
    pub span: Span,
}

impl Ty {
    pub fn new(kind: TyKind, span: Span) -> Self {
        Self {
            id: TyId::new(),
            kind,
            span,
        }
    }

    pub fn infer(span: Span) -> Self {
        Self::new(TyKind::Infer, span)
    }
}
```

### 2. Add `TyKind::Infer`

```rust
// ty/kind.rs
pub enum TyKind {
    // ... existing variants ...

    /// Type to be inferred (replaces TypeVar)
    Infer,
}
```

### 3. Add `ExprId` to `Expression`

```rust
// expr.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(u64);

impl ExprId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        ExprId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

pub struct Expression {
    pub id: ExprId,     // NEW (or existing)
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
    // ...
}
```

### 4. Remove `TypeVarId`

The existing `TypeVarId` can be removed (or deprecated) once `TyKind::Infer` + `TyId` is in place.

## Implementation Order

1. **Add `TyId` to `Ty`** in `kestrel-semantic-tree`
   - Add `TyId` struct with atomic counter
   - Add `id` field to `Ty`
   - Update `Ty::new()` to generate ID
   - Update all call sites

2. **Add `TyKind::Infer`** variant
   - Add variant to `TyKind`
   - Add `Ty::infer(span)` constructor
   - Update `Display` impl

3. **Create `kestrel-semantic-type-inference` crate**
   - Set up Cargo.toml with dependencies
   - Create module structure

4. **Implement `Constraint` and `Solution`** types
   - `constraint.rs` with Constraint enum
   - `solution.rs` with Solution and ValueResolution

5. **Implement `TypeOracle` trait**
   - `oracle.rs` with trait and MemberResolution

6. **Implement `InferenceContext`**
   - `context.rs` with constraint collection methods
   - Builder pattern for adding constraints

7. **Implement solver**
   - `solver.rs` with unification algorithm
   - Fixpoint iteration loop
   - Occurs check
   - Constraint-specific solving (Equals, Conforms, Normalizes, MemberAccess)

8. **Implement errors**
   - `error.rs` with InferenceError enum
   - Integrate with `kestrel-reporting` if needed

9. **Write tests**
   - Unit tests with mock oracle
   - Test unification cases
   - Test fixpoint iteration
   - Test error cases (occurs check, type mismatch, etc.)

## Future Considerations

- **Performance**: The fixpoint loop is O(n * k) where n is constraints and k is iterations. May need optimization for large programs.
- **Error recovery**: Currently fails on first error. Could collect multiple errors.
- **Diagnostics**: Rich error messages with type provenance ("expected Int because of line X").
- **Integration**: Constraint generation in binder, solution application after solving.
