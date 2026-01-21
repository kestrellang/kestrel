# Key File Locations

## By Task

| Task | File Path |
|------|-----------|
| Add token/keyword | `lib/kestrel-lexer/src/lib.rs` |
| Add syntax node kind | `lib/kestrel-syntax-tree/src/lib.rs` |
| Add parser for feature | `lib/kestrel-parser/src/{feature}/mod.rs` |
| Add to declaration items | `lib/kestrel-parser/src/declaration_item/mod.rs` |
| Shared parser utilities | `lib/kestrel-parser/src/common/` |
| Expression parsing | `lib/kestrel-parser/src/expr/mod.rs` |
| Statement parsing | `lib/kestrel-parser/src/stmt/mod.rs` |
| Add semantic symbol | `lib/kestrel-semantic-tree/src/symbol/{name}.rs` |
| Add symbol kind | `lib/kestrel-semantic-tree/src/symbol/kind.rs` |
| Add behavior | `lib/kestrel-semantic-tree/src/behavior/{name}.rs` |
| Add builder (BUILD) | `lib/kestrel-semantic-tree-builder/src/builders/{name}.rs` |
| Register builder | `lib/kestrel-semantic-tree-builder/src/lowerer.rs` |
| Add binder (BIND) | `lib/kestrel-semantic-tree-binder/src/binders/{name}.rs` |
| Register binder | `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs` |
| Body resolution | `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs` |
| Type resolution | `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs` |
| Add analyzer (VALIDATE) | `lib/kestrel-semantic-analyzers/src/analyzers/{name}/mod.rs` |
| Register analyzer | `lib/kestrel-semantic-analyzers/src/lib.rs` |
| Primitive types | `lib/kestrel-prelude/src/lib.rs` |
| Add integration test | `lib/kestrel-test-suite/tests/{name}.rs` |

## Common Imports by Crate

### Lexer
```rust
use logos::Logos;
use kestrel_span::{Span, Spanned};
```

### Parser
```rust
use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use crate::event::{Event, EventSink, TreeBuilder};
```

### Semantic Tree
```rust
use std::sync::Arc;
use kestrel_span::{Name, Span, Spanned};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};
use crate::language::KestrelLanguage;
use crate::symbol::kind::KestrelSymbolKind;
```

### Tests
```rust
use kestrel_test_suite::{Test, Compiles, HasError, Symbol, SymbolKind, Behavior, Visibility};
```

For complete reference: `docs/contributing/quick-reference.md`
