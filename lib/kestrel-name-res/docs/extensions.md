# Extension Resolution

Extension resolution is deliberately separate from the main name resolution pipeline. This avoids cycles and keeps extension logic isolated.

## Queries

### ExtensionTargetEntity

`ExtensionTargetEntity { extension, root }` — resolves an extension's target `AstType` to the entity it extends.

1. Checks for a pre-resolved `ResolvedExtensionTarget` component (performance optimization for known targets)
2. Falls back to resolving the `ExtensionTarget(AstType)` component via `ResolveTypePath`
3. Resolution context is the extension's parent module (not the extension itself)

**No cycle risk**: Type resolution never looks through extensions — they lack the `Typed` marker, so `ResolveTypePath` won't consider them as type results.

### ExtensionsFor

`ExtensionsFor { target, root }` — finds all extensions in the world that target a given type entity.

Performs a DFS through the entire module hierarchy from root, collecting `Extension`-kinded entities whose resolved target matches. Returns `Vec<Entity>`.

## Integration with Value Resolution

Extensions integrate into `ResolveValuePath` as a fallback: when resolving the last segment of a multi-segment value path, if no direct children match, extension static methods are checked. This is the only point where extensions participate in name resolution.
