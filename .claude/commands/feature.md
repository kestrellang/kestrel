---
description: Complete feature implementation workflow with brainstorm, design, planning, and implementation phases.
model: opus
---

You are implementing a new feature for the Kestrel compiler. This workflow has multiple confirmation gates to ensure quality.

# Reference

!`cat .claude/partials/compiler-architecture.md`

---

# Workflow Phases

## Phase 1: Brainstorm (Exploration)

Launch a subagent to explore the codebase and gather context:

```
Use Task tool with subagent_type="Explore" to find:
- Similar features already implemented (patterns to follow)
- Files that would be affected by this feature
- Potential edge cases and error conditions
- Feature interactions with existing language features
- Test patterns for similar features
```

After exploration, engage in Socratic discussion:
- Ask probing questions about edge cases
- Challenge assumptions constructively
- Present alternatives found in the codebase
- Discuss error handling approaches
- Validate consistency with existing patterns

**CONFIRMATION GATE 1**: Ask user to confirm design direction before proceeding.

---

## Phase 2: Design Document

Create `docs/plans/{feature_name}/{feature_name}-design.md` containing:

```markdown
# {Feature Name} Design

## Overview
Brief description and motivation

## Syntax
```kestrel
// Example syntax
```

## Semantic Behavior
- What it means
- How it interacts with other features

## Error Cases
| Condition | Error Message |
|-----------|---------------|
| ... | ... |

## Edge Cases
- List edge cases and how they're handled

## Open Questions (Resolved)
- Questions that came up during brainstorm and their resolutions
```

**CONFIRMATION GATE 2**: User confirms design document before planning.

---

## Phase 3: Implementation Plan

Create `docs/plans/{feature_name}/{feature_name}-plan.md` containing:

```markdown
# {Feature Name} Implementation Plan

## Test Strategy
- Test categories to write
- Key behaviors to verify
- Error cases to test

## Implementation Phases

### Phase 0: Tests (First!)
Files: lib/kestrel-test-suite/tests/{feature}.rs
- [ ] Basic tests
- [ ] Visibility tests (if applicable)
- [ ] Error case tests
- [ ] Edge case tests

### Phase 1: Lexer (if new tokens)
Files: lib/kestrel-lexer/src/lib.rs
- [ ] Add token(s)

### Phase 2: Syntax Tree
Files: lib/kestrel-syntax-tree/src/lib.rs
- [ ] Add SyntaxKind variants
- [ ] Update kind_from_raw()

### Phase 3: Parser
Files: lib/kestrel-parser/src/{feature}/mod.rs
- [ ] Create parser module
- [ ] Integrate into declaration_item

### Phase 4: Semantic Symbol
Files: lib/kestrel-semantic-tree/src/symbol/
- [ ] Add to KestrelSymbolKind
- [ ] Create symbol struct

### Phase 5: Builder (BUILD)
Files: lib/kestrel-semantic-tree-builder/src/builders/
- [ ] Create builder
- [ ] Register in lowerer.rs

### Phase 6: Binder (BIND)
Files: lib/kestrel-semantic-tree-binder/src/binders/
- [ ] Create binder
- [ ] Register in declaration_binder.rs

### Phase 7: Validation (if needed)
Files: lib/kestrel-semantic-analyzers/src/analyzers/
- [ ] Create analyzer
- [ ] Register in lib.rs

## Verification
- [ ] All tests pass: cargo test
- [ ] Linting clean: cargo clippy
- [ ] Formatted: cargo fmt
```

Reference `docs/contributing/workflows.md` for standard patterns.

**CONFIRMATION GATE 3**: User confirms plan before implementation.

---

## Phase 4: Implementation

Implement each phase in order, running tests after each:

1. **Tests First**: Write failing tests that define expected behavior
2. **Lexer**: Add tokens if needed
3. **Parser**: Create parser, integrate into declaration_item (CRITICAL: add to .or() chain!)
4. **Semantic Symbol**: Add symbol kind and create symbol
5. **Builder**: Create builder, register in lowerer.rs
6. **Binder**: Create binder, register in declaration_binder.rs
7. **Validation**: Add analyzers if needed

After each phase:
```bash
cargo test
```

Report progress to user. Continue only if tests pass.

!`cat .claude/partials/common-pitfalls.md`

---

## Phase 5: Documentation

After all tests pass:

1. **User docs**: Write `docs/language/{feature}.md` describing the feature for users

2. **AI guide**: Update `docs/ai-kestrel-guide.md` with examples for AI code generation if needed

3. **Update tracking**:
   - Mark completed items in `TODO.md`
   - Check off items in `ROADMAP.md`

---

# Feature to Implement

$ARGUMENTS

Begin with **Phase 1: Brainstorm** by launching an exploration subagent.
