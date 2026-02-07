# Creating Analyzers (Validation Passes)

This document explains how to add validation to the Kestrel compiler.

Historically, validations lived as “passes” in the semantic-tree builder. The compiler now uses:

- **Build (lowering)**: `kestrel-semantic-tree-builder` builds a `SemanticModel` from syntax trees.
- **Bind**: `kestrel-semantic-tree-binder` resolves names/types, attaches relationships, and performs a small number of bind-time checks.
- **Analyze**: `kestrel-semantic-analyzers` runs validators as analyzers over the bound model.

## Where analyzers live

Analyzers are in `lib/kestrel-semantic-analyzers/src/analyzers/`.

The “standard set” and ordering is defined in `lib/kestrel-semantic-analyzers/src/lib.rs` via `default_analyzers()`.

## When to write an analyzer vs binder check

Prefer an **analyzer** when the rule:

- is purely validation (no mutation required),
- can run after binding,
- benefits from queries (e.g. “find all functions”, “resolve call target”, “is symbol visible”).

Prefer a **binder check** when the rule:

- must run while resolving bodies/names (e.g. to avoid creating invalid semantic nodes),
- requires attaching additional data to symbols/expressions during binding.

## Process for creating a new analyzer

### Step 1: Check for overlap

Search for an existing analyzer first:

- Look under `lib/kestrel-semantic-analyzers/src/analyzers/`
- Check `default_analyzers()` ordering in `lib/kestrel-semantic-analyzers/src/lib.rs`
- Grep for an existing diagnostic type/message substring

### Step 2: Define the errors

Before writing code, define exactly what the analyzer detects. Create a table:

| Condition | Error Message |
|-----------|---------------|
| When X happens | "error message describing X" |
| When Y happens | "error message describing Y" |

Keep messages stable; tests usually assert on substrings.

### Step 3: Implement the analyzer

Create a new module under `lib/kestrel-semantic-analyzers/src/analyzers/<your_rule>/`:

```rust
use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

pub struct MyAnalyzer;

impl MyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &'static str {
        "my_analyzer"
    }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
        // Use ctx.model.query(...) to fetch derived information.
        // Use ctx.report(...) to emit diagnostics.
        let _ = (symbol, ctx);
    }
}
```

Define diagnostics in a sibling `diagnostics.rs` file and implement `kestrel_reporting::IntoDiagnostic` on your error type(s).

### Step 4: Register it

Add it to `default_analyzers()` in `lib/kestrel-semantic-analyzers/src/lib.rs` in the appropriate order.

### Step 5: Add tests

Add tests under `lib/kestrel-test-suite/tests/` (follow existing patterns in the suite).

Run:

```bash
cargo test -p kestrel-test-suite
```

### Accessing Behaviors

Get behavior data from symbols:

```rust
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;

fn get_some_behavior(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<&SomeBehavior> {
    let behaviors = symbol.metadata().behaviors();
    behaviors
        .iter()
        .find(|b| matches!(b.kind(), KestrelBehaviorKind::SomeBehavior))
        .and_then(|b| b.as_ref().downcast_ref::<SomeBehavior>())
}
```

### Checking Symbol Kinds

```rust
let kind = symbol.metadata().kind();

match kind {
    KestrelSymbolKind::Function => { /* ... */ }
    KestrelSymbolKind::Struct => { /* ... */ }
    KestrelSymbolKind::Protocol => { /* ... */ }
    KestrelSymbolKind::Enum => { /* ... */ }
    KestrelSymbolKind::Extension => { /* ... */ }
    KestrelSymbolKind::TypeAlias => { /* ... */ }
    KestrelSymbolKind::Field => { /* ... */ }
    KestrelSymbolKind::Module => { /* ... */ }
    KestrelSymbolKind::SourceFile => { /* ... */ }
    _ => {}
}
```

### Collecting Children for Duplicate Detection

```rust
use std::collections::HashMap;

fn check_duplicates(scope: &Arc<dyn Symbol<KestrelLanguage>>) {
    let mut seen: HashMap<String, Arc<dyn Symbol<KestrelLanguage>>> = HashMap::new();

    for child in scope.metadata().children() {
        let name = child.metadata().name().value.clone();

        if let Some(first) = seen.get(&name) {
            // Report duplicate error
        } else {
            seen.insert(name, child.clone());
        }
    }
}
```

## Tips

1. **Skip the root symbol** - The semantic tree root is a placeholder. Skip it when checking for type-specific rules.

2. **Use debug mode** - Support `config.debug_mode` to include pass name in errors for debugging.

3. **Test both valid and invalid cases** - Ensure valid code compiles and invalid code produces the expected error.

4. **Check existing passes for patterns** - The existing analyzers in `analyzers/` demonstrate common patterns.

5. **Consider the symbol hierarchy** - Understand parent-child relationships:
   - Root > Module > SourceFile > declarations
   - Struct/Enum/Protocol > members (fields, functions)
