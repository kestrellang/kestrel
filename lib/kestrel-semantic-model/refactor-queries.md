# Query Refactoring Plan

This document outlines refactoring tasks to improve query consistency and composition.

## Principles

- Queries take `SymbolId`, not `Arc<dyn Symbol>`
- Queries compose by calling other queries via `model.query(...)`
- No direct `model.registry().get()` calls - use `SymbolFor` instead
- Deduplicate helper functions by converting to queries

---

## 1. Replace `registry().get()` with `SymbolFor`

Every direct `model.registry().get(id)` should become `model.query(SymbolFor { id })`.

| File | Lines |
|------|-------|
| `scope_for.rs` | 29-30 |
| `imports_in_scope.rs` | 25-27 |
| `child_by_name.rs` | 24 |
| `visible_children.rs` | 25 |
| `is_visible_from.rs` | 22, 26 |
| `resolve_name.rs` | 55 |
| `resolve_type_path.rs` | 54, 72, 108, 153, 165, 237, 264, 307, 358, 393 |
| `resolve_value_path.rs` | 39, 59, 64 |

---

## 2. Create `InheritedProtocolMember` Query

Deduplicates `find_in_inherited_protocols` which is copy-pasted in:
- `resolve_name.rs:98-128`
- `resolve_type_path.rs:411-441`

```rust
/// Search inherited protocols for a member (e.g., associated type).
pub struct InheritedProtocolMember {
    pub protocol_id: SymbolId,
    pub name: String,
}
// Output: Option<SymbolId>
```

After creating, update both `resolve_name.rs` and `resolve_type_path.rs` to use this query.

---

## 3. Create `VisibleChildrenByName` Query

Used in multiple places via `visibility::find_visible_children_by_name()`.

```rust
/// Find children of parent that are visible from context and match name.
pub struct VisibleChildrenByName {
    pub parent: SymbolId,
    pub name: String,
    pub context: SymbolId,
}
// Output: Vec<Arc<dyn Symbol<KestrelLanguage>>>
```

Composes: `SymbolFor`, `IsVisibleFrom`

Update call sites:
- `resolve_type_path.rs:128-129`
- `resolve_value_path.rs:122-126`

---

## 4. Create `AncestorOfKind` Query

Used in visibility checking for module lookups.

```rust
/// Find the nearest ancestor of a specific kind.
pub struct AncestorOfKind {
    pub symbol_id: SymbolId,
    pub kind: KestrelSymbolKind,
}
// Output: Option<SymbolId>
```

Update `visibility.rs` (or inline into `IsVisibleFrom`).

---

## 5. Delete `visibility.rs`

After creating the above queries:
- `find_visible_children_by_name` → `VisibleChildrenByName` query
- `find_ancestor_of_kind` → `AncestorOfKind` query
- `is_visible_from` → inline into `IsVisibleFrom` query

The module can be deleted once all functions are converted.

---

## 6. Keep as Functions (No Change)

These helpers don't need model access or are only used locally:

| File | Function | Reason |
|------|----------|--------|
| `resolve_type_path.rs` | `resolve_primitive_type` | Pure function, no model needed |
| `resolve_value_path.rs` | `extract_value_from_symbols` | Operates on already-loaded symbols |

---

## Verification

After each change, run:
```bash
cd /Users/dino/Documents/Projects/kestrel && cargo build && cargo test
```
