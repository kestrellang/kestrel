# Semantic Model Migration

Migration from `SemanticDatabase` + `Db` trait to `SemanticModel` + `Query` trait.

## Status

- [x] Step 1: Create crate, move types
- [ ] Step 2: Add Query trait + SemanticModel struct
- [ ] Step 3: Implement core queries
- [ ] Step 4: Implement resolution queries
- [ ] Step 5: Update binder
- [ ] Step 6: Update resolvers
- [ ] Step 7: Update body resolver
- [ ] Step 8: Update validation passes
- [ ] Step 9: Update kestrel-compiler
- [ ] Step 10: Cleanup

---

## Step 1: Create Crate + Move Types ✓

Create `kestrel-semantic-model` crate. Move from builder:
- `Scope`, `Import`, `ImportItem`
- `SymbolResolution`, `TypePathResolution`, `ValuePathResolution`
- `SymbolRegistry`, `ExtensionRegistry`

Builder re-exports these for backwards compatibility.

## Step 2: Add Query Trait + SemanticModel

In `kestrel-semantic-model`:

```rust
pub trait Query {
    type Output;
    fn execute(self, model: &SemanticModel) -> Self::Output;
}

pub struct SemanticModel {
    root: Arc<dyn Symbol<KestrelLanguage>>,
    syntax_map: HashMap<SymbolId, SyntaxNode>,
    sources: HashMap<String, String>,
    registry: SymbolRegistry,
    extension_registry: ExtensionRegistry,
}

impl SemanticModel {
    pub fn query<Q: Query>(&self, q: Q) -> Q::Output;
}
```

## Step 3: Implement Core Queries

Foundational queries (L0-L1):
- `SymbolById { id }` → `Option<Arc<dyn Symbol>>`
- `ExtensionsFor { target_id }` → `Vec<Arc<ExtensionSymbol>>`
- `ScopeFor { symbol_id }` → `Arc<Scope>`

## Step 4: Implement Resolution Queries

Move logic from `SemanticDatabase` impl (L2-L4):
- `ResolveName { name, context }`
- `ResolveModulePath { path, context }`
- `ResolveTypePath { path, context }`
- `ResolveValuePath { path, context }`
- `IsVisibleFrom { target, context }`
- `VisibleChildrenOf { parent, context }`
- `FindChildByName { parent, name }`

## Step 5: Update Binder

Change `SemanticBinder` to take and return `SemanticModel`:

```rust
impl SemanticBinder {
    pub fn bind(model: SemanticModel, diagnostics: &mut DiagnosticContext) -> SemanticModel {
        // ... run binding using model.query(...) ...
        model
    }
}
```

Remove `SemanticDatabase` field.

## Step 6: Update Resolvers

Change resolver signatures from `db: &dyn Db` to `model: &SemanticModel`.

Files: `module.rs`, `struct.rs`, `function.rs`, `field.rs`, `protocol.rs`, `extension.rs`, `type_alias.rs`, `initializer.rs`, `associated_type.rs`, `import.rs`

## Step 7: Update Body Resolver

Change `BodyResolutionContext`:
```rust
pub struct BodyResolutionContext<'a> {
    model: &'a SemanticModel,  // was: db: &'a dyn Db
}
```

Update all `ctx.db.method()` calls to `ctx.model.query(...)`.

## Step 8: Update Validation Passes

Change validation pass signatures:
```rust
fn validate(
    &self,
    root: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,  // was: db: &SemanticDatabase
    diagnostics: &mut DiagnosticContext,
    config: &ValidationConfig,
)
```

## Step 9: Update kestrel-compiler

```rust
pub struct Compilation {
    source_files: Vec<SourceFile>,
    semantic_model: Option<SemanticModel>,  // was: semantic_tree
    diagnostics: DiagnosticContext,
}
```

Add `semantic_model()` method. Remove `semantic_tree()`.

## Step 10: Cleanup

Delete from builder:
- `database/semantic_db.rs`
- `database/queries.rs` (just the `Db` trait remains, then delete)
- Re-exports of moved types

Remove any remaining legacy semantic-tree types/APIs once nothing depends on them.
