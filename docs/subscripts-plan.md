# Subscripts Implementation Plan

## Overview

Add subscripts to Kestrel with full parser and semantic support. Subscripts are like computed properties but accept parameters, enabling indexed or keyed access patterns like `array(0)` or `dictionary("key")`.

**Design decisions:**
- Uses `subscript` keyword (not a named property)
- Call-syntax with parentheses: `array(0)`, `array(safe: 0)`
- Supports labeled parameters for overloading variants
- Swift-style body: `{ get { } set { } }` or shorthand `{ expr }`
- Implicit `newValue` parameter in setters
- Same visibility for getter/setter (no split)
- Allowed in: structs, enums, protocols, extensions
- Static subscripts supported
- Generic subscripts with where clauses supported

## Architecture

### Semantic Tree Structure

```
SubscriptSymbol (subscript(index: Int) -> T)
  ├── TypeParameterSymbol (if generic: [K])
  ├── GetterSymbol (CallableBehavior + ExecutableBehavior)
  └── SetterSymbol (optional, CallableBehavior + ExecutableBehavior)
```

- Subscript is the parent symbol (like a field with computed property)
- Getter/setter are child symbols with callable + executable behaviors
- Parameters live on the SubscriptSymbol itself (not getter/setter)

### Flow

```
Source: array(0)
   ↓
Parser: ExprCall { callee: ExprPath("array"), args: [0] }
   ↓
Resolver: Detect callee is a value with subscript → find matching subscript
   ↓
Semantic: Call { callee: SubscriptGetter, args: [0], receiver: array }
   ↓
Lowering: Generate getter call with receiver and args
```

### Assignment Flow

```
Source: array(0) = value
   ↓
Parser: ExprAssign { target: ExprCall(...), value }
   ↓
Resolver: Detect assignment target is subscript call → use setter
   ↓
Semantic: Call { callee: SubscriptSetter, args: [0, newValue], receiver: array }
   ↓
Lowering: Generate setter call
```

---

## Phase 1: Parser

### 1.1 Add SyntaxKinds

**File:** `lib/kestrel-syntax-tree/src/lib.rs`

Add to SyntaxKind enum:
```rust
SubscriptDeclaration,  // subscript(index: Int) -> T { ... }
SubscriptBody,         // { expr } or { get { } set { } }
```

Note: Reuse existing `GetterClause`, `SetterClause`, `PropertyAccessors` from computed properties.

### 1.2 Add Subscript Keyword Token

**File:** `lib/kestrel-lexer/src/lib.rs` (if not already present)

Check if `subscript` keyword exists. If not, add:
```rust
"subscript" => SyntaxKind::Subscript,
```

### 1.3 Create SubscriptDeclarationData

**File:** `lib/kestrel-parser/src/common/data.rs`

```rust
pub struct SubscriptDeclarationData {
    pub visibility: Option<Span>,
    pub static_modifier: Option<Span>,
    pub subscript_keyword: Span,
    pub type_parameters: Option<TypeParameterListData>,
    pub parameters: ParameterListData,
    pub return_type: ReturnTypeData,
    pub where_clause: Option<WhereClauseData>,
    pub body: SubscriptBodyData,
}

pub enum SubscriptBodyData {
    /// Shorthand: `{ expr }`
    Shorthand(CodeBlockData),
    /// Explicit: `{ get { } set { } }`
    Accessors {
        getter: Option<CodeBlockData>,  // None for protocol `{ get }`
        setter: Option<CodeBlockData>,  // None for protocol `{ get set }`
    },
    /// Protocol requirement: `{ get }` or `{ get set }`
    Requirement { has_setter: bool },
}
```

### 1.4 Create Subscript Parser

**New file:** `lib/kestrel-parser/src/subscript/mod.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl SubscriptDeclaration {
    pub fn visibility(&self) -> Option<SyntaxNode> { /* find Visibility child */ }
    pub fn is_static(&self) -> bool { /* check StaticModifier */ }
    pub fn type_parameters(&self) -> Option<SyntaxNode> { /* find TypeParameterList */ }
    pub fn parameters(&self) -> Option<SyntaxNode> { /* find ParameterList */ }
    pub fn return_type(&self) -> Option<SyntaxNode> { /* find ReturnType */ }
    pub fn where_clause(&self) -> Option<SyntaxNode> { /* find WhereClause */ }
    pub fn body(&self) -> Option<SyntaxNode> { /* find SubscriptBody */ }
    pub fn getter_body(&self) -> Option<SyntaxNode> { /* find GetterClause body */ }
    pub fn setter_body(&self) -> Option<SyntaxNode> { /* find SetterClause body */ }
    pub fn is_getter_only(&self) -> bool { /* no setter clause */ }
    pub fn is_protocol_requirement(&self) -> bool { /* body is { get } or { get set } */ }
}

pub fn parse_subscript_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    // Parse: visibility? static? SUBSCRIPT type_params? LPAREN params RPAREN ARROW type where? body
}
```

### 1.5 Add to Declaration Router

**File:** `lib/kestrel-parser/src/declaration_item/mod.rs`

Add to DeclarationItem enum:
```rust
pub enum DeclarationItem {
    // ... existing ...
    Subscript(SubscriptDeclaration),
}
```

Update the parser to recognize `subscript` keyword as declaration starter.

### 1.6 Add Emitter Function

**File:** `lib/kestrel-parser/src/common/emitters.rs`

```rust
pub fn emit_subscript_declaration(sink: &mut EventSink, data: SubscriptDeclarationData) {
    sink.start_node(SyntaxKind::SubscriptDeclaration);

    // Visibility
    emit_visibility(sink, data.visibility);

    // Static modifier (optional)
    if let Some(static_span) = data.static_modifier {
        sink.start_node(SyntaxKind::StaticModifier);
        sink.add_token(SyntaxKind::Static, static_span);
        sink.finish_node();
    }

    // Subscript keyword
    sink.add_token(SyntaxKind::Subscript, data.subscript_keyword);

    // Type parameters (optional)
    if let Some(type_params) = data.type_parameters {
        emit_type_parameter_list(sink, type_params);
    }

    // Parameters (required)
    emit_parameter_list(sink, data.parameters);

    // Return type (required)
    emit_return_type(sink, data.return_type);

    // Where clause (optional)
    if let Some(where_clause) = data.where_clause {
        emit_where_clause(sink, where_clause);
    }

    // Body
    emit_subscript_body(sink, data.body);

    sink.finish_node();
}

fn emit_subscript_body(sink: &mut EventSink, body: SubscriptBodyData) {
    sink.start_node(SyntaxKind::SubscriptBody);
    match body {
        SubscriptBodyData::Shorthand(code_block) => {
            emit_code_block(sink, code_block);
        }
        SubscriptBodyData::Accessors { getter, setter } => {
            sink.start_node(SyntaxKind::PropertyAccessors);
            if let Some(getter_body) = getter {
                emit_getter_clause(sink, getter_body);
            }
            if let Some(setter_body) = setter {
                emit_setter_clause(sink, setter_body);
            }
            sink.finish_node();
        }
        SubscriptBodyData::Requirement { has_setter } => {
            // Protocol requirement: { get } or { get set }
            sink.start_node(SyntaxKind::PropertyAccessors);
            sink.add_token(SyntaxKind::Get, /* span */);
            if has_setter {
                sink.add_token(SyntaxKind::Set, /* span */);
            }
            sink.finish_node();
        }
    }
    sink.finish_node();
}
```

---

## Phase 2: Semantic Symbols

### 2.1 Add SubscriptSymbol

**New file:** `lib/kestrel-semantic-tree/src/symbol/subscript.rs`

```rust
use crate::prelude::*;

#[derive(Debug)]
pub struct SubscriptSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    is_static: bool,
    locals: RwLock<Vec<Local>>,
}

impl SubscriptSymbol {
    pub fn new(
        span: Span,
        declaration_span: Span,
        visibility: VisibilityBehavior,
        is_static: bool,
        parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    ) -> Self {
        let metadata = SymbolMetadata::new(
            KestrelSymbolKind::Subscript,
            "subscript".to_string(),  // Synthetic name
            span,
            declaration_span,
            parent,
        );
        metadata.add_behavior(visibility);

        Self {
            metadata,
            is_static,
            locals: RwLock::new(Vec::new()),
        }
    }

    pub fn is_static(&self) -> bool {
        self.is_static
    }

    /// Get the getter child symbol
    pub fn getter(&self) -> Option<Arc<GetterSymbol>> {
        self.metadata.children().iter()
            .find_map(|c| c.clone().downcast::<GetterSymbol>().ok())
    }

    /// Get the setter child symbol (if exists)
    pub fn setter(&self) -> Option<Arc<SetterSymbol>> {
        self.metadata.children().iter()
            .find_map(|c| c.clone().downcast::<SetterSymbol>().ok())
    }
}

impl Symbol<KestrelLanguage> for SubscriptSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl LocalContainer for SubscriptSymbol {
    fn locals(&self) -> &RwLock<Vec<Local>> {
        &self.locals
    }
}
```

### 2.2 Update KestrelSymbolKind

**File:** `lib/kestrel-semantic-tree/src/symbol/kind.rs`

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum KestrelSymbolKind {
    // ... existing ...
    Subscript,
}
```

### 2.3 Export Symbol

**File:** `lib/kestrel-semantic-tree/src/symbol/mod.rs`

```rust
pub mod subscript;
pub use subscript::SubscriptSymbol;
```

---

## Phase 3: Builder

### 3.1 Create SubscriptBuilder

**New file:** `lib/kestrel-semantic-tree-builder/src/builders/subscript.rs`

```rust
use crate::prelude::*;

pub struct SubscriptBuilder;

impl Builder for SubscriptBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        file_id: usize,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        let parent = parent?;

        // Subscripts only valid in struct, enum, protocol, extension
        let parent_kind = parent.metadata().kind();
        if !matches!(parent_kind,
            KestrelSymbolKind::Struct
            | KestrelSymbolKind::Protocol
            | KestrelSymbolKind::Enum
            | KestrelSymbolKind::Extension)
        {
            return None;
        }

        // Extract visibility
        let visibility = extract_visibility_behavior(syntax, parent);

        // Check for static
        let is_static = has_static_modifier(syntax);

        // Get spans
        let full_span = get_node_span(syntax, file_id);
        let decl_span = find_subscript_keyword_span(syntax, file_id);

        // Create subscript symbol
        let subscript = SubscriptSymbol::new(
            full_span,
            decl_span,
            visibility,
            is_static,
            Some(parent.clone()),
        );
        let subscript_arc = Arc::new(subscript);
        let subscript_dyn = subscript_arc.clone() as Arc<dyn Symbol<KestrelLanguage>>;

        // Build type parameters as children (if generic)
        if let Some(type_params_node) = find_child(syntax, SyntaxKind::TypeParameterList) {
            build_type_parameters(&type_params_node, source, file_id, &subscript_dyn);
        }

        // Build getter/setter children
        if let Some(body) = find_child(syntax, SyntaxKind::SubscriptBody) {
            build_subscript_accessors(&body, source, file_id, &subscript_dyn);
        }

        // Register with parent
        parent.metadata().add_child(&subscript_dyn);

        Some(subscript_arc)
    }
}

fn build_subscript_accessors(
    body: &SyntaxNode,
    source: &str,
    file_id: usize,
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
) {
    // Check for PropertyAccessors (explicit get/set)
    if let Some(accessors) = find_child(body, SyntaxKind::PropertyAccessors) {
        // Build getter
        if let Some(getter_clause) = find_child(&accessors, SyntaxKind::GetterClause) {
            let getter = GetterSymbol::new(/* ... */);
            parent.metadata().add_child(&(Arc::new(getter) as Arc<dyn Symbol<_>>));
        }

        // Build setter (optional)
        if let Some(setter_clause) = find_child(&accessors, SyntaxKind::SetterClause) {
            let setter = SetterSymbol::new(/* ... */);
            parent.metadata().add_child(&(Arc::new(setter) as Arc<dyn Symbol<_>>));
        }
    } else {
        // Shorthand body - getter only
        let getter = GetterSymbol::new(/* ... */);
        parent.metadata().add_child(&(Arc::new(getter) as Arc<dyn Symbol<_>>));
    }
}
```

### 3.2 Register Builder

**File:** `lib/kestrel-semantic-tree-builder/src/lowerer.rs`

In `builder_for()`:
```rust
fn builder_for(kind: SyntaxKind) -> Option<Box<dyn Builder>> {
    match kind {
        // ... existing ...
        SyntaxKind::SubscriptDeclaration => Some(Box::new(SubscriptBuilder)),
        // ...
    }
}
```

### 3.3 Export Builder

**File:** `lib/kestrel-semantic-tree-builder/src/builders/mod.rs`

```rust
pub mod subscript;
pub use subscript::SubscriptBuilder;
```

---

## Phase 4: Binder

### 4.1 Create SubscriptBinder

**New file:** `lib/kestrel-semantic-tree-binder/src/binders/subscript.rs`

```rust
pub struct SubscriptBinder;

impl Binder for SubscriptBinder {
    fn bind_signature(&self, ctx: &mut BinderContext) {
        let subscript = ctx.symbol.downcast_ref::<SubscriptSymbol>().unwrap();
        let syntax = ctx.syntax();

        // Resolve parameter types
        let parameters = resolve_parameters(ctx, syntax);

        // Resolve return type
        let return_type = resolve_return_type(ctx, syntax);

        // Determine receiver type
        let receiver = if subscript.is_static() {
            ReceiverKind::None
        } else {
            ReceiverKind::Borrowing  // Getter borrows self
        };

        // Add SubscriptBehavior (holds parameter info for overload resolution)
        let subscript_behavior = SubscriptBehavior::new(
            parameters.clone(),
            return_type.clone(),
            receiver,
        );
        ctx.symbol.metadata().add_behavior(subscript_behavior);

        // Bind getter signature
        if let Some(getter) = subscript.getter() {
            bind_getter_signature(ctx, &getter, &parameters, &return_type, receiver);
        }

        // Bind setter signature (if present)
        if let Some(setter) = subscript.setter() {
            let setter_receiver = if subscript.is_static() {
                ReceiverKind::None
            } else {
                ReceiverKind::Mutating  // Setter mutates self
            };
            bind_setter_signature(ctx, &setter, &parameters, &return_type, setter_receiver);
        }
    }

    fn bind_body(&self, ctx: &mut BinderContext) {
        let subscript = ctx.symbol.downcast_ref::<SubscriptSymbol>().unwrap();

        // Bind getter body
        if let Some(getter) = subscript.getter() {
            bind_getter_body(ctx, &getter);
        }

        // Bind setter body
        if let Some(setter) = subscript.setter() {
            bind_setter_body(ctx, &setter);
        }
    }
}

fn bind_getter_signature(
    ctx: &mut BinderContext,
    getter: &Arc<GetterSymbol>,
    parameters: &[Parameter],
    return_type: &Ty,
    receiver: ReceiverKind,
) {
    // Getter has same parameters as subscript, returns subscript's return type
    let callable = CallableBehavior::new(
        parameters.to_vec(),
        return_type.clone(),
        receiver,
    );
    getter.metadata().add_behavior(callable);
}

fn bind_setter_signature(
    ctx: &mut BinderContext,
    setter: &Arc<SetterSymbol>,
    parameters: &[Parameter],
    return_type: &Ty,
    receiver: ReceiverKind,
) {
    // Setter has subscript params + newValue param, returns Unit
    let mut setter_params = parameters.to_vec();
    setter_params.push(Parameter {
        label: None,
        name: "newValue".to_string(),
        ty: return_type.clone(),
        has_default: false,
    });

    let callable = CallableBehavior::new(
        setter_params,
        Ty::unit(),
        receiver,
    );
    setter.metadata().add_behavior(callable);
}
```

### 4.2 Add SubscriptBehavior

**New file:** `lib/kestrel-semantic-tree/src/behavior/subscript.rs`

```rust
/// Behavior for subscript declarations - enables overload resolution
#[derive(Debug, Clone)]
pub struct SubscriptBehavior {
    parameters: Vec<Parameter>,
    return_type: Ty,
    receiver: ReceiverKind,
}

impl SubscriptBehavior {
    pub fn new(parameters: Vec<Parameter>, return_type: Ty, receiver: ReceiverKind) -> Self {
        Self { parameters, return_type, receiver }
    }

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn return_type(&self) -> &Ty {
        &self.return_type
    }

    pub fn receiver(&self) -> ReceiverKind {
        self.receiver
    }

    /// Check if this subscript matches the given arguments
    pub fn matches(&self, args: &[Argument]) -> bool {
        // Check argument count and labels match parameters
        if args.len() != self.parameters.len() {
            return false;
        }

        for (arg, param) in args.iter().zip(&self.parameters) {
            // Check label matches
            if arg.label != param.label {
                return false;
            }
        }

        true
    }
}

impl Behavior<KestrelLanguage> for SubscriptBehavior {
    fn as_any(&self) -> &dyn std::any::Any { self }
}
```

### 4.3 Register Binder

**File:** `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs`

In binder registry:
```rust
fn binder_for(kind: KestrelSymbolKind) -> Option<Box<dyn Binder>> {
    match kind {
        // ... existing ...
        KestrelSymbolKind::Subscript => Some(Box::new(SubscriptBinder)),
        // ...
    }
}
```

### 4.4 Export Binder

**File:** `lib/kestrel-semantic-tree-binder/src/binders/mod.rs`

```rust
pub mod subscript;
pub use subscript::SubscriptBinder;
```

---

## Phase 5: Call Resolution

### 5.1 Update Member Resolution

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

When resolving a call on a value (e.g., `array(0)`), check for subscripts:

```rust
fn resolve_value_call(
    ctx: &mut ResolverContext,
    callee: &Expression,
    args: &[Argument],
) -> Option<Expression> {
    let callee_type = callee.ty();

    // Get the type's symbol
    let type_symbol = ctx.lookup_type_symbol(&callee_type)?;

    // Look for matching subscript
    let subscripts = type_symbol.metadata().children()
        .iter()
        .filter_map(|c| c.clone().downcast::<SubscriptSymbol>().ok())
        .collect::<Vec<_>>();

    // Find subscript that matches arguments
    for subscript in subscripts {
        if let Some(behavior) = subscript.metadata().behavior::<SubscriptBehavior>() {
            if behavior.matches(args) {
                // Found matching subscript - generate getter call
                let getter = subscript.getter()?;
                return Some(Expression::call(
                    Expression::symbol_ref(getter),
                    args.to_vec(),
                    Some(callee.clone()),  // receiver
                ));
            }
        }
    }

    None
}
```

### 5.2 Update Assignment Resolution

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/assignments.rs`

When assigning to a subscript call (e.g., `array(0) = value`):

```rust
fn resolve_assignment(
    ctx: &mut ResolverContext,
    target: &Expression,
    value: &Expression,
) -> Option<Expression> {
    // Check if target is a subscript call
    if let Expression::Call { callee, args, receiver, .. } = target {
        if let Some(getter) = callee.as_symbol_ref::<GetterSymbol>() {
            // Find the parent subscript
            let subscript = getter.metadata().parent()?
                .downcast::<SubscriptSymbol>().ok()?;

            // Get setter (error if none)
            let setter = subscript.setter().ok_or_else(|| {
                ctx.error("cannot assign to read-only subscript")
            })?;

            // Generate setter call with newValue
            let mut setter_args = args.clone();
            setter_args.push(Argument {
                label: None,
                value: value.clone(),
            });

            return Some(Expression::call(
                Expression::symbol_ref(setter),
                setter_args,
                receiver.clone(),
            ));
        }
    }

    // ... handle other assignment targets ...
}
```

---

## Phase 6: Semantic Analysis

### 6.1 Subscript Validation Pass

**New file:** `lib/kestrel-semantic-analyzers/src/subscript_validation.rs`

```rust
pub struct SubscriptValidationPass;

impl AnalysisPass for SubscriptValidationPass {
    fn run(&self, ctx: &mut AnalysisContext) {
        for symbol in ctx.symbols_of_kind(KestrelSymbolKind::Subscript) {
            let subscript = symbol.downcast_ref::<SubscriptSymbol>().unwrap();

            // Check: subscript must have at least one parameter
            if let Some(behavior) = subscript.metadata().behavior::<SubscriptBehavior>() {
                if behavior.parameters().is_empty() {
                    ctx.error(
                        subscript.metadata().span(),
                        "subscript must have at least one parameter",
                    );
                }

                // Check: no default values on parameters
                for param in behavior.parameters() {
                    if param.has_default {
                        ctx.error(
                            subscript.metadata().span(),
                            "subscript parameters cannot have default values",
                        );
                    }
                }
            }

            // Check: must have body (unless protocol requirement)
            if !subscript.is_protocol_requirement() {
                if subscript.getter().is_none() {
                    ctx.error(
                        subscript.metadata().span(),
                        "subscript must have a body",
                    );
                }
            }
        }
    }
}
```

### 6.2 Assignment Validation

Extend existing assignment validation to handle subscript setters:

```rust
// In assignment validation pass
fn validate_assignment_target(ctx: &mut AnalysisContext, target: &Expression) {
    if let Expression::Call { callee, .. } = target {
        if let Some(getter) = callee.as_symbol_ref::<GetterSymbol>() {
            let subscript = getter.metadata().parent()
                .and_then(|p| p.downcast::<SubscriptSymbol>().ok());

            if let Some(subscript) = subscript {
                if subscript.setter().is_none() {
                    ctx.error(
                        target.span(),
                        "cannot assign to read-only subscript",
                    );
                }
            }
        }
    }
}
```

---

## Phase 7: Protocol Conformance

### 7.1 Subscript Requirement Matching

When checking protocol conformance, match subscript requirements:

```rust
fn check_subscript_conformance(
    ctx: &mut ConformanceContext,
    requirement: &SubscriptSymbol,
    impl_type: &TypeSymbol,
) -> bool {
    let req_behavior = requirement.metadata().behavior::<SubscriptBehavior>()?;

    // Find matching subscript in implementation
    let impl_subscripts = impl_type.subscripts();

    for impl_subscript in impl_subscripts {
        let impl_behavior = impl_subscript.metadata().behavior::<SubscriptBehavior>()?;

        // Check parameters match
        if !parameters_match(req_behavior.parameters(), impl_behavior.parameters()) {
            continue;
        }

        // Check return type matches
        if !types_match(req_behavior.return_type(), impl_behavior.return_type()) {
            continue;
        }

        // Check getter/setter requirements
        let req_has_setter = requirement.setter().is_some();
        let impl_has_setter = impl_subscript.setter().is_some();

        if req_has_setter && !impl_has_setter {
            // Requirement needs setter but impl doesn't have one
            continue;
        }

        return true;  // Found matching implementation
    }

    false
}
```

---

## Files to Modify/Create

### Parser
- `lib/kestrel-syntax-tree/src/lib.rs` - Add SyntaxKinds
- `lib/kestrel-lexer/src/lib.rs` - Add `subscript` keyword (if needed)
- `lib/kestrel-parser/src/common/data.rs` - Add SubscriptDeclarationData
- `lib/kestrel-parser/src/common/emitters.rs` - Add emit functions
- `lib/kestrel-parser/src/subscript/mod.rs` - NEW: Parser
- `lib/kestrel-parser/src/declaration_item/mod.rs` - Route subscript declarations

### Semantic Tree
- `lib/kestrel-semantic-tree/src/symbol/mod.rs` - Export
- `lib/kestrel-semantic-tree/src/symbol/kind.rs` - Add Subscript kind
- `lib/kestrel-semantic-tree/src/symbol/subscript.rs` - NEW: SubscriptSymbol
- `lib/kestrel-semantic-tree/src/behavior/mod.rs` - Export
- `lib/kestrel-semantic-tree/src/behavior/subscript.rs` - NEW: SubscriptBehavior

### Builder
- `lib/kestrel-semantic-tree-builder/src/builders/mod.rs` - Export
- `lib/kestrel-semantic-tree-builder/src/builders/subscript.rs` - NEW: SubscriptBuilder
- `lib/kestrel-semantic-tree-builder/src/lowerer.rs` - Register builder

### Binder
- `lib/kestrel-semantic-tree-binder/src/binders/mod.rs` - Export
- `lib/kestrel-semantic-tree-binder/src/binders/subscript.rs` - NEW: SubscriptBinder
- `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs` - Register binder
- `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs` - Resolve subscript calls
- `lib/kestrel-semantic-tree-binder/src/body_resolver/assignments.rs` - Handle subscript assignment

### Analyzers
- `lib/kestrel-semantic-analyzers/src/mod.rs` - Register pass
- `lib/kestrel-semantic-analyzers/src/subscript_validation.rs` - NEW: Validation

---

## Verification

```bash
# Parser tests
cargo test -p kestrel-parser subscript

# Symbol tests
cargo test -p kestrel-semantic-tree subscript

# Binder tests
cargo test -p kestrel-semantic-tree-binder subscript

# Integration - uncomment stdlib subscripts and check they compile
cargo run -- check lang/std/collections/array.ks
cargo run -- check lang/std/memory/buffer.ks
cargo run -- check lang/std/collections/dictionary.ks

# Full test suite
cargo test
```

---

## Test Cases

### Parser Tests
```kestrel
// Shorthand getter
subscript(index: Int) -> T { self.data(index) }

// Explicit getter
subscript(index: Int) -> T { get { self.data(index) } }

// Getter + setter
subscript(index: Int) -> T {
    get { self.data(index) }
    set { self.data(index) = newValue }
}

// Labeled parameter
subscript(safe index: Int) -> Optional[T] { ... }

// Multiple parameters
subscript(row: Int, column: Int) -> T { ... }

// Generic subscript
subscript[K](key: K) -> Optional[V] where K: Hashable { ... }

// Static subscript
static subscript(key: String) -> T { ... }

// Protocol requirement
subscript(index: Int) -> T { get }
subscript(index: Int) -> T { get set }
```

### Call Resolution Tests
```kestrel
let array = Array[Int]()
let x = array(0)           // Resolves to getter
array(0) = 42              // Resolves to setter
let y = array(safe: 0)     // Resolves to labeled subscript
let z = matrix(0, 1)       // Multi-param subscript
```

### Validation Tests
```kestrel
// Error: no parameters
subscript() -> T { ... }

// Error: default value
subscript(index: Int = 0) -> T { ... }

// Error: no body (outside protocol)
subscript(index: Int) -> T

// Error: assign to read-only
let x = array(0)  // OK
array(0) = 42     // Error if getter-only
```

---

## Design Decisions

1. **Subscript naming** - Subscripts use synthetic name "subscript" since they're accessed by signature, not name

2. **Call syntax** - Uses parentheses `array(0)` not brackets `array[0]` - consistent with Kestrel's unified call syntax

3. **Overload resolution** - Subscripts overload by parameter labels and types, like functions

4. **Getter/setter as children** - Reuses GetterSymbol/SetterSymbol from computed properties

5. **Protocol requirements** - Use `{ get }` or `{ get set }` syntax like computed properties

6. **No default parameters** - Subscript parameters cannot have defaults (simplifies overload resolution)

7. **Static subscripts** - Supported for type-level access patterns (e.g., caches)
