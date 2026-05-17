# Scope Construction

Scopes are the foundation of name resolution. Each declaration entity gets a `Scope` that contains all names available at that level — local declarations, imports, and a parent link for chain walking.

## Scope Structure

```rust
struct Scope {
    selective_imports: HashMap<String, Vec<Entity>>,  // from `import A.B.(Foo)` or `import A.B as X`
    declarations: HashMap<String, Vec<Entity>>,       // local child declarations
    wildcard_imports: Vec<Entity>,                     // source modules for `import A.B.*`
    parent: Option<Entity>,                            // parent scope for chain walk
    entity: Entity,                                    // the entity this scope belongs to
}
```

## ScopeFor Query

`ScopeFor { entity }` builds the scope for any entity. The query:

1. **Classifies children** by `NodeKind`:
   - Import declarations → processed as imports
   - Everything else → added to `declarations` by name

2. **Processes imports** by type:
   - **Selective** (`import A.B.(Foo, Bar)`) → resolves module path, looks up named items, adds to `selective_imports`
   - **Aliased** (`import A.B as X`) → resolves module path, adds module entity under alias name in `selective_imports`
   - **Wildcard** (`import A.B.*`) → resolves module path, adds to `wildcard_imports`

3. **Auto-imports std**: Non-stdlib entities automatically get all std leaf modules added as wildcard imports. Stdlib entities skip this (checked via `is_in_std_module()`).

4. **Sets parent**: Links to the entity's parent for scope chain walking.

Returns `Arc<Scope>` for query caching.

## Auto-Import Mechanism

All non-stdlib code automatically sees every public declaration from the standard library. This works by:

1. `StdModules` query collects all **leaf** submodules of `std` (modules with no child modules)
2. `ScopeFor` adds these leaf modules as wildcard imports for non-stdlib entities
3. During name resolution, wildcard imports are searched (with visibility filtering)

This means user code can write `Array[Int64]` without any import — `Array` is found via the auto-imported `std.collections.array` leaf module.

## Import Resolution

Import processing resolves the module path first via `ResolveModulePath`, then handles the import kind:

- **Selective**: For each named item in the import list, finds matching visible children in the resolved module
- **Aliased**: Maps the alias name to the module entity itself
- **Wildcard**: Stores the module entity for lazy resolution during name lookup

If a module path fails to resolve, that import is silently skipped (errors are reported elsewhere).
