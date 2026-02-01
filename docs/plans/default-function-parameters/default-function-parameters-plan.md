# Default Function Parameters Implementation Plan

## Test Strategy

### Test Categories
1. **Basic defaults**: Simple literal defaults (`x: Int = 0`)
2. **Expression defaults**: Complex expressions (`timestamp: Date = Date.now()`)
3. **Labeled parameters with defaults**: `func create(with name: String = "default")`
4. **Multiple defaults**: All params defaulted, partial defaults
5. **Calling with defaults**: Omitting args, providing all args, partial args
6. **Ordering validation**: Error when required param follows default
7. **Overload interaction**: Duplicate detection with defaults
8. **Initializers**: Default params in `init()`
9. **Subscripts**: Default params in `subscript()`
10. **Generic functions**: Defaults with type parameters

### Key Behaviors to Verify
- Default expressions evaluated at call site (fresh per call)
- Type checking of default expressions
- Call resolution with fewer arguments
- Label matching with omitted defaulted parameters
- Proper error messages for violations

## Implementation Phases

### Phase 0: Tests (First!)
**Files**: `lib/kestrel-test-suite/tests/default_parameters.rs`

- [ ] Basic default parameter syntax parses
- [ ] Function call with omitted default args
- [ ] Function call with all args provided
- [ ] Multiple default parameters
- [ ] Default with expression (not just literal)
- [ ] Labeled parameter with default
- [ ] Error: required param after default param
- [ ] Error: duplicate function with same signature (defaults ignored)
- [ ] Initializer with default params
- [ ] Subscript with default params

### Phase 1: Syntax Tree
**Files**: `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `DefaultValue` to `SyntaxKind` enum (for `= expression` in parameters)
- [ ] Add constant and match arm in `kind_from_raw()`

### Phase 2: Parser Data Structures
**Files**: `lib/kestrel-parser/src/common/data.rs`

- [ ] Add `default: Option<(Span, ExprVariant)>` field to `ParameterData`
  - Span is for the `=` token, ExprVariant is the default expression

### Phase 3: Parser
**Files**: `lib/kestrel-parser/src/common/parsers.rs`

- [ ] Update `parse_parameter()` to parse optional `= expression` after type
- [ ] Emit `DefaultValue` node containing `=` token and expression
- [ ] Handle in both function parameters and subscript parameters

### Phase 4: Semantic Tree - Parameter
**Files**: `lib/kestrel-semantic-tree/src/symbol/function.rs`

- [ ] Add `default_value: Option<SyntaxNode>` to `Parameter` struct
  - Store syntax node (not resolved expression) for call-site evaluation
- [ ] Update `Parameter` constructors if needed

### Phase 5: Semantic Tree - CallableParameter
**Files**: `lib/kestrel-semantic-tree/src/behavior/callable.rs`

- [ ] Add `has_default: bool` to `CallableParameter`
  - We only need to know IF there's a default for signature matching
  - The actual expression is stored in `Parameter` and evaluated at call site
- [ ] Update constructors: `new()`, `with_label()`, `with_access_mode()`, etc.
- [ ] Add `has_default()` accessor method

### Phase 6: Builder (BUILD Phase)
**Files**: `lib/kestrel-semantic-tree-builder/src/builders/function.rs`

- [ ] Extract `DefaultValue` child from Parameter syntax node
- [ ] Store syntax node reference in built `Parameter`
- [ ] Same for initializer builder and subscript builder

### Phase 7: Binder (BIND Phase)
**Files**:
- `lib/kestrel-semantic-tree-binder/src/binders/utils/parameters.rs`
- `lib/kestrel-semantic-tree-binder/src/binders/function.rs`

- [ ] Update `resolve_single_parameter()` to extract default value syntax node
- [ ] Set `has_default` on `CallableParameter` based on presence of default
- [ ] Type-check default expression matches parameter type
  - Create a resolution context and resolve the default expression
  - Verify assignability to parameter type

### Phase 8: Validation (Ordering Check)
**Files**: `lib/kestrel-semantic-tree-binder/src/binders/function.rs` (or new analyzer)

- [ ] After resolving all parameters, validate ordering:
  - Once a parameter has a default, all following must also have defaults
  - Emit error: "required parameter cannot follow a parameter with a default value"
- [ ] Apply same validation to initializers and subscripts

### Phase 9: Call Resolution
**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs`

- [ ] Update `matches_signature()` to allow fewer arguments if:
  - The missing arguments correspond to parameters with defaults
  - All omitted params are at the end OR identified by label
- [ ] New matching logic:
  ```
  for each param:
    if arg exists with matching label: match
    else if arg exists positionally: match
    else if param has_default: ok (use default)
    else: no match
  ```

### Phase 10: Argument Filling at Call Site
**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [ ] When call matches but has fewer args than params:
  - For each missing parameter with a default:
    - Resolve the default expression in the callee's context
    - Create a synthetic `CallArgument` with the resolved expression
    - Insert into the argument list at the correct position
- [ ] Handle labeled calls: match by label, fill defaults for missing

### Phase 11: Expression Resolution for Defaults
**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [ ] When evaluating a default at call site:
  - Get the syntax node from the Parameter
  - Create resolution context with caller's type substitutions (for generics)
  - Resolve the expression to get the `Expression`
  - Type-check against the (substituted) parameter type

### Phase 12: Diagnostics
**Files**: `lib/kestrel-semantic-tree-binder/src/diagnostics/mod.rs`

- [ ] Add `RequiredParamAfterDefaultError`:
  - "required parameter '{name}' cannot follow parameter '{prev}' which has a default value"
- [ ] Add `DefaultTypeMismatchError`:
  - "default value of type '{actual}' cannot be assigned to parameter type '{expected}'"

## Verification

After implementation:
- [ ] All tests pass: `cargo test -p kestrel-test-suite`
- [ ] Full test suite: `cargo test`
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

## Files Summary

| Phase | Files to Modify |
|-------|-----------------|
| Tests | `lib/kestrel-test-suite/tests/default_parameters.rs` (new) |
| Syntax | `lib/kestrel-syntax-tree/src/lib.rs` |
| Parser Data | `lib/kestrel-parser/src/common/data.rs` |
| Parser | `lib/kestrel-parser/src/common/parsers.rs` |
| Semantic Param | `lib/kestrel-semantic-tree/src/symbol/function.rs` |
| Semantic Callable | `lib/kestrel-semantic-tree/src/behavior/callable.rs` |
| Builder | `lib/kestrel-semantic-tree-builder/src/builders/function.rs` |
| Binder | `lib/kestrel-semantic-tree-binder/src/binders/utils/parameters.rs` |
| Binder | `lib/kestrel-semantic-tree-binder/src/binders/function.rs` |
| Call Resolution | `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs` |
| Call Resolution | `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs` |
| Diagnostics | `lib/kestrel-semantic-tree-binder/src/diagnostics/mod.rs` |

## Notes

- The default expression syntax node is stored, not the resolved expression, because:
  1. Call-site evaluation requires resolving with the caller's context
  2. Generic type parameters need substitution at call site
  3. Expressions like `Date.now()` must evaluate fresh per call

- Signature matching for overloads ignores defaults - two functions with same name/labels are duplicates even if one has defaults

- Labeled parameters can be omitted anywhere if they have defaults; unlabeled defaults must be at the end
