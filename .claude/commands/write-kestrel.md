---
description: Write Kestrel programs with guidance from the AI Kestrel guide.
model: sonnet
---

You are helping write Kestrel code.

# Reference Documentation

**Primary**: Read `docs/ai-kestrel-guide.md` for:
- Language syntax and semantics
- Idiomatic patterns
- Common mistakes to avoid
- Standard library usage
- Examples

**Secondary**: Read `docs/language/` for:
- Feature specifications
- Detailed syntax reference

---

# Guidelines

## Code Style

1. **Visibility**: Use appropriate visibility modifiers
   - `public` - accessible from other modules
   - `private` - only within the same type (default for struct members)
   - `internal` - within the same module tree
   - `fileprivate` - within the same file

2. **Mutability**: Prefer immutable bindings
   - `let` - immutable (preferred)
   - `var` - mutable (only when needed)

3. **Naming**:
   - Types: `PascalCase` (structs, protocols, enums)
   - Functions/methods: `camelCase`
   - Variables: `camelCase`
   - Constants: `SCREAMING_SNAKE_CASE`

4. **Self parameter**:
   - `self` - borrowing (default, read-only access)
   - `mutating self` - mutating (can modify fields)
   - `consuming self` - takes ownership

---

# Process

1. Read the user's request
2. Check `docs/ai-kestrel-guide/` for relevant patterns
3. Write idiomatic Kestrel code
4. Explain any design decisions

---

# Request

$ARGUMENTS

Write the Kestrel code, explaining any design decisions.
