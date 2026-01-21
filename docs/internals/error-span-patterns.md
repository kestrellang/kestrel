# Error Span Patterns

This document defines the conventions for span usage in Kestrel error messages to ensure consistent, precise, and helpful diagnostics.

## Core Principles

1. **Primary span** = The exact thing that's wrong (red highlight)
2. **Secondary span** = Context that helps understand the error (blue highlight)
3. **Prefer specific over general** - Point to `PrivateClass` not `import Library.(PrivateClass)`
4. **Cross-file when needed** - Show declarations in other files with correct file_id

## Span Categories

### 1. Name Spans
Use for: Identifiers, symbol names, type names

```
import Library.(PrivateClass)
                ^^^^^^^^^^^^  ← name span (just the identifier)
```

**When to use:**
- Symbol not found → primary on the missing name
- Symbol not visible → primary on the inaccessible name
- Duplicate declaration → primary on the conflicting name
- Type mismatch → primary on the type name

### 2. Declaration Spans
Use for: The full extent of a declaration

```
private struct PrivateStruct {}
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  ← declaration span
```

**When to use:**
- "Declared here" secondary labels
- Showing where something was defined
- Multiple declaration errors

### 3. Segment Spans
Use for: Individual parts of a dotted path

```
import Math.Geometry.Point
       ^^^^                 ← segment 0 span
            ^^^^^^^^        ← segment 1 span
                     ^^^^^  ← segment 2 span
```

**When to use:**
- Module not found → primary on the failing segment
- Nested path resolution errors

### 4. Full Path Spans
Use for: An entire dotted path expression

```
import Math.Geometry.Point
       ^^^^^^^^^^^^^^^^^^^  ← path span
```

**When to use:**
- Secondary context for "in this import"
- When the whole path is relevant

### 5. Item Spans
Use for: Individual items in a list (import items, parameters, etc.)

```
import Library.(Point, Circle, Triangle)
                ^^^^^                     ← item span for Point
                       ^^^^^^             ← item span for Circle
                               ^^^^^^^^   ← item span for Triangle
```

**When to use:**
- Specific import item errors
- Parameter errors in function calls
- List element type mismatches

---

## Error Type Patterns

### ModuleNotFoundError

```
error: module 'NonExistent' not found
  ┌─ file.ks:1:8
  │
1 │ import NonExistent.Foo
  │        ^^^^^^^^^^^-----
  │        │
  │        no module named 'NonExistent'
  │        in this import
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `failed_segment_span` | Segment | The specific segment that failed (primary) |
| `path_span` | Full Path | The entire module path (secondary) |

---

### SymbolNotFoundInModuleError

```
error: symbol 'Foo' not found in module 'Library'
  ┌─ file.ks:1:17
  │
1 │ import Library.(Foo)
  │ ----------------^^^-
  │ │               │
  │ │               'Foo' does not exist
  │ in module 'Library'
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `symbol_span` | Name/Item | The missing symbol name (primary) |
| `module_span` | Full Path | The module path for context (secondary) |

---

### SymbolNotVisibleError

```
error: 'PrivateClass' is not accessible
   ┌─ consumer.ks:5:17
   │
 5 │ import Library.(PrivateClass)
   │                 ^^^^^^^^^^^^ 'PrivateClass' is private
   │
   ┌─ library.ks:11:15
   │
11 │ private struct PrivateStruct {}
   │               ------------ 'PrivateClass' declared as private here
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `import_span` | Name/Item | The inaccessible symbol in import (primary) |
| `declaration_span` | Name | The symbol's name in its declaration (secondary, cross-file) |
| `declaration_file_id` | - | File containing the declaration |

---

### ImportConflictError

```
error: 'Point' is already imported
   ┌─ file.ks:8:1
   │
 8 │ import Math.Geometry
   │ ^^^^^^^^^^^^^^^^^^^^ cannot import 'Point'
   ·
11 │ import Math.Geometry.(Point, Circle)
   │                       ----- 'Point' first imported here
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `import_span` | Declaration | The conflicting import statement (primary) |
| `existing_span` | Name/Item | The specific item that was first imported (secondary) |

---

### TypeMismatchError (Future)

```
error: type mismatch
   ┌─ file.ks:5:12
   │
 5 │ let x: Int = "hello"
   │        ^^^   ^^^^^^^ expected `Int`, found `String`
   │        │
   │        expected due to this type annotation
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `found_span` | Expression | The expression with wrong type (primary) |
| `expected_span` | Name | The type annotation (secondary) |

---

### DuplicateDeclarationError (Future)

```
error: duplicate declaration of 'Foo'
   ┌─ file.ks:5:7
   │
 5 │ struct Foo {}
   │       ^^^ 'Foo' redeclared here
   ·
 2 │ struct Foo {}
   │       --- first declared here
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `duplicate_span` | Name | The duplicate name (primary) |
| `original_span` | Name | The original declaration's name (secondary) |

---

### GenericConstraintError (Future - Phase 2)

```
error: type 'Int' does not satisfy constraint 'Comparable'
   ┌─ file.ks:10:15
   │
10 │ let sorted = sort<Int>(numbers)
   │                   ^^^ `Int` does not implement `Comparable`
   ·
 3 │ fn sort<T: Comparable>(items: Array<T>) -> Array<T>
   │            ---------- required by this constraint
```

| Field | Span Type | Points To |
|-------|-----------|-----------|
| `type_arg_span` | Name | The type argument that fails (primary) |
| `constraint_span` | Name | The constraint definition (secondary) |

---

## Implementation Checklist

When implementing a new error type:

- [ ] Identify what's **wrong** → that gets the primary span
- [ ] Identify what provides **context** → those get secondary spans
- [ ] Use **name spans** for symbols, not full declaration spans
- [ ] Store **file_id** for cross-file references
- [ ] Store **segment index** for path resolution errors
- [ ] Include **spans for each item** in list-based constructs

## Span Extraction Utilities

```rust
// From ImportItem - use item.span for the specific name
let item_span = item.span.clone();

// From Symbol - use name span for precise pointing
let name_span = symbol.metadata().name().span.clone();

// From ModulePath - use segments_with_spans() for segment-level precision
let segments = module_path.segments_with_spans();
let segment_span = segments[index].1.clone();

// For cross-file diagnostics
let file_id = ctx.file_id_for_symbol(&symbol);
```
