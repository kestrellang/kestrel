# String Interpolation Implementation Plan

## Test Strategy

### Test Categories
1. **Basic interpolation** - `"\(x)"`, `"Hello \(name)!"`
2. **Format specifiers** - `"\(x:>8)"`, `"\(n:08x)"`, `"\(pi:.2)"`
3. **Complex expressions** - `"\(a + b)"`, `"\(obj.method())"`, `"\(arr[0])"`
4. **Nested interpolation** - `"\(a + "\(b)")"`
5. **Nested strings** - `"\(dict["key"])"`
6. **Edge cases** - empty literals, consecutive interpolations, escaped backslash
7. **Error cases** - unterminated, invalid specs, non-Formattable types
8. **Protocol conformance** - custom types with `ExpressibleByStringInterpolation`
9. **Raw strings** - verify no interpolation in `"""..."""`

### Key Behaviors to Verify
- Correct string output for various format specs
- Compile-time errors for invalid format specs
- Type checking (Formattable conformance)
- Proper span tracking for error messages
- Nested string/interpolation handling

---

## Implementation Phases

### Phase 0: Standard Library Types
**Files:**
- `lang/std/text/format.ks` (new)
- `lang/std/core/protocols.ks` (update)
- `lang/std/text/string.ks` (update)

**Tasks:**
- [ ] Create `FormatOptions` struct
- [ ] Create `Alignment`, `Sign`, `FloatStyle` enums
- [ ] Create `StringInterpolationProtocol` protocol
- [ ] Create `ExpressibleByStringInterpolation` protocol
- [ ] Create `DefaultStringInterpolation` struct
- [ ] Update `Formattable` protocol: `format(options: FormatOptions = .default) -> String`
- [ ] Update all `Formattable` implementations (Int, Float, Bool, String, etc.)
- [ ] Implement `ExpressibleByStringInterpolation` for `String`
- [ ] Add `concat(parts: [String]) -> String` function

---

### Phase 1: Lexer
**Files:**
- `lib/kestrel-lexer/src/lib.rs`

**Tasks:**
- [ ] Add `LexerMode` enum: `Normal`, `InString`, `InInterpolation`
- [ ] Add mode stack to lexer extras
- [ ] Add new tokens:
  - `StringStart` - opening `"` of interpolated string
  - `StringPart` - literal text segment
  - `InterpolationStart` - `\(`
  - `FormatSpec` - `:...` format specifier
  - `InterpolationEnd` - `)` closing interpolation
  - `StringEnd` - closing `"`
- [ ] Implement `lex_string_start` callback for `"`
- [ ] Implement stateful lexing:
  - In `InString`: accumulate text into `StringPart`, handle `\(`, handle closing `"`
  - In `InInterpolation`: track bracket depths, handle `:` for format spec, handle nested `"`
- [ ] Handle escaped backslash: `\\(` should not start interpolation
- [ ] Keep simple `String` token for non-interpolated strings (optimization)

**Verification:**
```bash
cargo test -p kestrel-lexer
```

---

### Phase 2: Syntax Tree
**Files:**
- `lib/kestrel-syntax-tree/src/lib.rs`

**Tasks:**
- [ ] Add `SyntaxKind` variants:
  - `StringStart`
  - `StringPart`
  - `InterpolationStart`
  - `FormatSpec`
  - `InterpolationEnd`
  - `StringEnd`
  - `ExprInterpolatedString` - the overall interpolated string expression
  - `InterpolatedStringPart` - wrapper for each literal/interpolation part
  - `StringInterpolation` - wrapper for `\(expr:spec)` segment
- [ ] Update `kind_from_raw()` for new variants

**Verification:**
```bash
cargo test -p kestrel-syntax-tree
```

---

### Phase 3: Parser
**Files:**
- `lib/kestrel-parser/src/expr/mod.rs`
- `lib/kestrel-parser/src/expr/string_interpolation.rs` (new)
- `lib/kestrel-parser/src/common/emitters.rs`

**Tasks:**
- [ ] Create `string_interpolation.rs` module
- [ ] Implement `parse_interpolated_string`:
  - Consume `StringStart`
  - Loop: `StringPart` or interpolation
  - For interpolation: `InterpolationStart`, parse expression, optional `FormatSpec`, `InterpolationEnd`
  - Consume `StringEnd`
- [ ] Add `ExprVariant::InterpolatedString`
- [ ] Update expression parser to handle `StringStart` token
- [ ] Add emitter functions:
  - `emit_interpolated_string_expr`
  - `emit_string_interpolation`
  - `emit_interpolated_string_part`

**CST Structure:**
```
ExprInterpolatedString
├── StringStart
├── InterpolatedStringPart
│   └── StringPart("Hello ")
├── InterpolatedStringPart
│   └── StringInterpolation
│       ├── InterpolationStart
│       ├── Expr (the expression)
│       ├── FormatSpec (optional)
│       └── InterpolationEnd
├── InterpolatedStringPart
│   └── StringPart("!")
└── StringEnd
```

**Verification:**
```bash
cargo test -p kestrel-parser
```

---

### Phase 4: Semantic Tree
**Files:**
- `lib/kestrel-semantic-tree/src/expr.rs`

**Tasks:**
- [ ] Add `InterpolationPart` enum:
  ```rust
  pub enum InterpolationPart {
      Literal(String, Span),
      Interpolation {
          expr: Box<Expression>,
          format_options: FormatOptions,
          span: Span,
      },
  }
  ```
- [ ] Add to `ExprKind`:
  ```rust
  InterpolatedString {
      parts: Vec<InterpolationPart>,
      target_type: Option<TypeId>,  // The ExpressibleByStringInterpolation type
  }
  ```
- [ ] Add `FormatOptions` representation (or reference std lib type)

**Verification:**
```bash
cargo test -p kestrel-semantic-tree
```

---

### Phase 5: Format Spec Parser
**Files:**
- `lib/kestrel-semantic-tree-binder/src/format_spec.rs` (new)

**Tasks:**
- [ ] Create `format_spec.rs` module
- [ ] Implement `parse_format_spec(spec: &str, span: Span) -> Result<FormatOptions, Diagnostic>`:
  - Parse `[[fill]align][sign][#][0][width][.precision][type]`
  - Validate type specifier
  - Return `FormatOptions` or error diagnostic
- [ ] Handle all format spec components:
  - Fill character (any char before align)
  - Align: `<`, `>`, `^`
  - Sign: `+`, `-`, ` `
  - Alternate form: `#`
  - Zero-pad: `0`
  - Width: integer
  - Precision: `.` + integer
  - Type: `b`, `o`, `x`, `X`, `e`, `E`, `f`, `%`, `?`

**Verification:**
```bash
cargo test -p kestrel-semantic-tree-binder
```

---

### Phase 6: Binder
**Files:**
- `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`
- `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs`

**Tasks:**
- [ ] Add handler for `ExprInterpolatedString` syntax node
- [ ] For each `StringInterpolation` part:
  - Recursively resolve the inner expression
  - Check expression type conforms to `Formattable`
  - Parse format spec (if present) using `parse_format_spec`
  - Validate format spec is compatible with expression type
- [ ] For `StringPart` parts:
  - Extract and unescape the literal text
- [ ] Determine target type:
  - From type context (annotation, parameter type, etc.)
  - Default to `String` if no context
- [ ] Check target type conforms to `ExpressibleByStringInterpolation`
- [ ] Build `ExprKind::InterpolatedString`

**Verification:**
```bash
cargo test -p kestrel-semantic-tree-binder
```

---

### Phase 7: Validation (Analyzers)
**Files:**
- `lib/kestrel-semantic-analyzers/src/analyzers/` (if needed)

**Tasks:**
- [ ] Add analyzer for format spec / type compatibility (if not done in binder)
- [ ] Validate format specs make sense for the type:
  - `x`, `X`, `b`, `o` only for integers
  - `e`, `E`, `f`, `%` only for floats
  - Width/precision for any type
  - `?` (debug) for any type

---

### Phase 8: Code Generation
**Files:**
- `lib/kestrel-execution-graph-lowering/src/expr.rs`
- `lib/kestrel-compiler/src/` (if needed)

**Tasks:**
- [ ] Handle `ExprKind::InterpolatedString` in expression lowering
- [ ] Generate code equivalent to:
  ```kestrel
  {
      var __interp = TargetType.Interpolation(
          literalCapacity: <total_literal_chars>,
          interpolationCount: <num_interpolations>
      )
      __interp.appendLiteral("literal1")
      __interp.appendInterpolation(expr1, options: <format_options1>)
      __interp.appendLiteral("literal2")
      // ...
      TargetType(interpolation: __interp)
  }
  ```
- [ ] Generate `FormatOptions` construction from parsed spec
- [ ] Handle optimization cases:
  - Simple string (no interpolation) → use `ExpressibleByStringLiteral`
  - Single interpolation, no literals → could simplify

**Verification:**
```bash
cargo test -p kestrel-execution-graph-lowering
cargo test -p kestrel-compiler
```

---

### Phase 9: Tests
**Files:**
- `lib/kestrel-test-suite/tests/expressions/string_interpolation.rs` (new)

**Tasks:**
- [ ] Basic interpolation tests
- [ ] Format specifier tests (all types)
- [ ] Complex expression tests
- [ ] Nested interpolation tests
- [ ] Error case tests
- [ ] Protocol conformance tests
- [ ] Integration tests with print/println

---

## Verification Checklist

After each phase:
```bash
cargo test
cargo clippy
cargo fmt --check
```

Final verification:
- [ ] All existing tests pass
- [ ] All new interpolation tests pass
- [ ] No clippy warnings
- [ ] Code formatted

---

## Risk Areas

| Area | Risk | Mitigation |
|------|------|------------|
| Lexer state machine | Complex, easy to miss edge cases | Extensive lexer-level tests |
| Nested strings | Recursive state tracking | Careful mode stack management |
| Format spec parsing | Many combinations | Comprehensive spec parser tests |
| Type checking | Formattable conformance | Clear error messages |
| Codegen | Complex desugaring | Test generated code structure |

---

## Dependencies

This feature depends on:
- Working generic methods (for `appendInterpolation[T: Formattable]`)
- Default parameter values (for `options: FormatOptions = .default`)
- Associated types (for `type Interpolation`)
- Protocol conformance checking

Verify these work before starting implementation.

---

## Estimated Complexity

| Phase | Complexity | Notes |
|-------|------------|-------|
| Phase 0 (Std Lib) | Medium | Many types to update |
| Phase 1 (Lexer) | High | Stateful lexing is tricky |
| Phase 2 (Syntax) | Low | Just adding variants |
| Phase 3 (Parser) | Medium | New parser for interpolation |
| Phase 4 (Semantic) | Low | Adding expression kinds |
| Phase 5 (Format Spec) | Medium | Mini-language parser |
| Phase 6 (Binder) | High | Type checking, validation |
| Phase 7 (Validation) | Low | Optional additional checks |
| Phase 8 (Codegen) | High | Complex desugaring |
| Phase 9 (Tests) | Medium | Comprehensive coverage |
