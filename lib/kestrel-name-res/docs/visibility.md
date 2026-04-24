# Visibility

Visibility is a separate query layer, decoupled from name resolution itself. This allows visibility checks to be reused and composed independently.

## Visibility Levels

Entities carry an optional `Vis` component:

| Modifier | Rule |
|----------|------|
| `Public` | Always visible |
| `Private` | Visible within parent scope and all descendants |
| `Fileprivate` | Visible only within the same file (`FileId` component) |
| `Internal` | Visible within the same top-level module subtree |
| None | Always visible (default for entities without `Vis`) |

## Queries

### IsVisibleFrom

`IsVisibleFrom { target, context }` — checks if `target` is visible from `context`.

- **Public / no Vis**: always `true`
- **Private**: walks ancestors of `context` to see if `target`'s parent is an ancestor (including self)
- **Fileprivate**: walks ancestors of both `target` and `context` to find their `FileId` components, compares them
- **Internal**: finds the top-level module (direct child of root) for both entities, checks they match

### VisibleChildrenByName

`VisibleChildrenByName { entity, name, context }` — finds children of `entity` matching `name` that are visible from `context`.

Combines `find_children_by_name()` with `IsVisibleFrom` filtering. Used by scope construction (for wildcard imports) and value path resolution.

## Helper Functions

- `find_file_id(entity)`: Walk ancestors to find the nearest `FileId` component
- `top_level_module(entity)`: Find the direct child of root containing this entity (for `Internal` visibility)
