# Extensions with Conformances - Implementation Plan

## Overview

This document describes the implementation plan for adding extension support to Kestrel. Extensions allow adding methods and protocol conformances to existing types.

**Syntax:**

```kestrel
extend Type: Protocol {
    func method() { ... }
}
```

---

## Design Decisions

### 1. Extension Symbol Architecture

✅ **Extensions ARE symbols (ExtensionSymbol)**

- ExtensionSymbol is a real symbol in the semantic tree
- Methods are children of the ExtensionSymbol (NOT added to target struct)
- ExtensionRegistry maps target types to extensions
- Method lookup searches: struct's children + all extensions targeting that struct

### 2. Availability

✅ **Global availability**

- Extensions registered globally in SemanticDatabase
- Available everywhere once defined, no import needed
- Individual methods within extensions have their own visibility modifiers

### 3. Type Parameter Model

**Key Insight:** Extensions **reference** struct's type parameters, not declare new ones

```kestrel
struct Box[T, U] { }

extension Box[T, Int] {  // T references Box's T, Int is concrete
    func getFirst() -> T { ... }  // T is in scope
}
```

**Rules:**

- ✅ Can use type parameter: `T` → refers to struct's `T`
- ✅ Can use concrete type: `Int`
- ✅ Can mix: `Box[T, Int]`
- ✅ Type parameters in scope via walking up to target type's parameters
- ✗ Cannot use unrelated type params: `U` not in scope if not referenced
- ✗ Cannot use associated types directly as slot fillers

**Examples:**

```kestrel
struct Pair[T, U] { var first: T, var second: U }

extension Pair[T, Int] {        // ✓ T is any type, U must be Int
    func getSecond() -> Int { return self.second }
}

extension Pair[Int, String] {   // ✓ Both concrete
    func describe() -> String { "int and string pair" }
}

extension Pair[T, U] {          // ✓ Both type parameters
    func swap() -> Pair[U, T] { ... }
}
```

### 4. Where Clause Model

```kestrel
struct Box[T] where T: Comparable { }

extension Box[T] where T: Equatable {
    // T is Comparable (inherited from struct)
    // T is also Equatable (additional constraint)
}
```

**Rules:**

- Extensions **inherit** all constraints from struct
- Extensions **can add** additional constraints via `where` clause
- Where clause references type parameters from target type

### 5. Method Applicability (Unification)

Extensions are applicable to a type if:

1. Type arguments unify with extension's target pattern
2. Where clause constraints are satisfied

```kestrel
extension Box[T, Int] where T: Equatable { }

Box[String, Int]    // ✓ Applicable if String: Equatable
Box[String, String] // ✗ Not applicable (Int ≠ String)
Box[Int, Int]       // ✓ Applicable if Int: Equatable
```

**Algorithm:**

```rust
fn is_extension_applicable(extension: &Extension, instantiated_ty: &Ty) -> bool {
    let ext_target = extension.target_ty();  // Box[T, Int]

    // 1. Unify: Can instantiated_ty match ext_target pattern?
    if !unify(ext_target, instantiated_ty) {
        return false;
    }

    // 2. Check where clause constraints are satisfied
    check_where_clause_satisfied(extension, instantiated_ty)
}
```

### 6. Conflict Resolution: Specialized Wins

When multiple extensions provide the same method:

```kestrel
extension Box[T, U] {
    func describe() -> String { "generic" }
}

extension Box[Int, String] {
    func describe() -> String { "specialized" }  // ✓ This wins
}

let b: Box[Int, String] = ...
b.describe()  // → "specialized"
```

**Priority Rules:**

1. **Specificity**: More concrete type arguments = higher priority
   - `Box[Int, String]` > `Box[T, String]` > `Box[T, U]`
2. **Count concrete types**: More concrete slots = more specific
3. **Same specificity = Error**: Ambiguous extension methods detected in VALIDATE phase

**Conflict Detection:**

- Happens during VALIDATE phase (not at call time)
- Error for duplicate method signatures with same specificity
- Specialized methods allowed to override generic ones

### 7. Visibility

- Extensions themselves are module-level entities
- `private` methods in extensions are private to that extension only
- `public` methods are visible everywhere (global availability)

### 8. Scope Resolution

Type parameters from target type are resolved by walking up:

1. Check local scope (function parameters, locals)
2. Check extension's children (methods)
3. Check extension's parent scope (module level)
4. **Check target type's type parameters** ← NEW

---

## Implementation Plan

### Phase 1: Tests (Write First) ✅ TDD

**New file:** `lib/kestrel-test-suite/tests/declarations/extensions.rs`

Write comprehensive tests covering:

- Basic extensions (add methods to structs)
- Extensions with conformances
- Multiple extensions on same type
- Generic extensions referencing type parameters
- Specialized extensions (concrete types)
- Specialized wins over generic
- Mixed type parameters (`Box[T, Int]`)
- Where clause constraints (inherited and added)
- Extension satisfies protocol requirements
- Conflict detection (duplicate methods)
- Error cases (extending non-structs, type param mismatches)

See detailed test cases in the full specification.

---

### Phase 2: Lexer & Syntax Tree

**Files to modify:**

1. `lib/kestrel-lexer/src/lib.rs`
2. `lib/kestrel-syntax-tree/src/lib.rs`

**Changes:**

```rust
// In lexer.rs - Add token (in "Declaration Keywords" section)
#[token("extend")]
Extend,

// In syntax-tree.rs - Add nodes
ExtensionDeclaration,
ExtensionBody,
```

---

### Phase 3: Parser

**New file:** `lib/kestrel-parser/src/extension/mod.rs`

```rust
pub struct ExtensionDeclarationData {
    pub extend_span: Span,
    pub target_type: TyExpressionData,  // Reuse type expression parser
    pub conformances: Option<ConformanceListData>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body_items: Vec<ExtensionBodyItem>,
    pub rbrace_span: Span,
}

pub enum ExtensionBodyItem {
    Function(FunctionDeclarationData),
    // Maybe later: Static(StaticVariableData),
}

pub fn extension_declaration_parser_internal() -> impl Parser<...> {
    token(Token::Extend)
        .then(ty_expression_parser())  // Box[T, Int]
        .then(conformance_list_parser().or_not())
        .then(where_clause_parser().or_not())
        .then(token(Token::LBrace))
        .then(extension_body_item_parser().repeated())
        .then(token(Token::RBrace))
        .map(|tuple| ExtensionDeclarationData { ... })
}
```

**Update:** `lib/kestrel-parser/src/declaration_item/mod.rs`

- Add extension parser to choice combinator
- Route `extend` keyword to extension parser

---

### Phase 4: Semantic Symbol & Behaviors

**New file:** `lib/kestrel-semantic-tree/src/symbol/extension.rs`

```rust
pub struct ExtensionSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for ExtensionSymbol {
    fn kind(&self) -> SymbolKind { SymbolKind::Extension }
    // ... standard impl
}

// Constructor
pub fn new(
    span: Span,
    visibility: VisibilityBehavior,
    parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
) -> Self { ... }
```

**New file:** `lib/kestrel-semantic-tree/src/behavior/extension_target.rs`

```rust
#[derive(Debug, Clone)]
pub struct ExtensionTargetBehavior {
    target_ty: Ty,  // The type being extended (e.g., Box[T, Int])
}

impl Behavior<KestrelLanguage> for ExtensionTargetBehavior {
    fn kind(&self) -> BehaviorKind {
        BehaviorKind::ExtensionTarget
    }
}

impl ExtensionTargetBehavior {
    pub fn new(target_ty: Ty) -> Self {
        Self { target_ty }
    }

    pub fn target_ty(&self) -> &Ty {
        &self.target_ty
    }
}
```

**Updates:**

- Add `Extension` to `SymbolKind` enum
- Add `ExtensionTarget` to `KestrelBehaviorKind` enum
- Export new types in `mod.rs` files

---

### Phase 5: Extension Registry

**New file:** `lib/kestrel-semantic-tree/src/extension_registry.rs`

```rust
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    // Map: struct/protocol SymbolId → Vec<ExtensionSymbol IDs>
    extensions_by_target: HashMap<SymbolId, Vec<SymbolId>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions_by_target: HashMap::new(),
        }
    }

    pub fn register(&mut self, target_id: SymbolId, extension_id: SymbolId) {
        self.extensions_by_target
            .entry(target_id)
            .or_default()
            .push(extension_id);
    }

    pub fn get_extensions_for(&self, target_id: SymbolId) -> &[SymbolId] {
        self.extensions_by_target
            .get(&target_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn all_extensions(&self) -> impl Iterator<Item = (SymbolId, &[SymbolId])> {
        self.extensions_by_target
            .iter()
            .map(|(k, v)| (*k, v.as_slice()))
    }
}
```

**Update:** `lib/kestrel-semantic-tree/src/database.rs` or equivalent

```rust
pub struct SemanticDatabase {
    symbols: HashMap<SymbolId, Arc<dyn Symbol>>,
    extension_registry: ExtensionRegistry,  // ADD THIS
    // ... other fields
}
```

---

### Phase 6: Extension Resolver

**New file:** `lib/kestrel-semantic-tree-builder/src/resolvers/extension.rs`

```rust
pub struct ExtensionResolver;

impl Resolver for ExtensionResolver {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // 1. Extract extension span
        let extension_decl = ExtensionDeclaration::cast(syntax)?;
        let span = extension_decl.span();

        // 2. Create ExtensionSymbol
        let extension = Arc::new(ExtensionSymbol::new(
            span,
            VisibilityBehavior::new(None, Visibility::Internal, Scope::Module),
            parent.cloned(),
        ));

        // 3. Parse body items, create FunctionSymbols as children
        if let Some(body) = extension_decl.body() {
            for item in body.items() {
                match item {
                    ExtensionBodyItem::Function(func_node) => {
                        // Use FunctionResolver to build function
                        let func_resolver = FunctionResolver;
                        if let Some(func_sym) = func_resolver.build_declaration(
                            func_node.syntax(),
                            source,
                            Some(&extension),
                            root,
                        ) {
                            extension.add_child(func_sym);
                        }
                    }
                }
            }
        }

        // 4. Return ExtensionSymbol
        Some(extension)
    }

    fn bind_declaration(
        &self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        syntax: &SyntaxNode,
        context: &mut BindingContext,
    ) {
        let extension = symbol.downcast_ref::<ExtensionSymbol>().unwrap();
        let extension_decl = ExtensionDeclaration::cast(syntax).unwrap();

        // 1. Resolve target type expression → Ty
        let target_ty = if let Some(ty_expr) = extension_decl.target_type() {
            resolve_type_expression(&ty_expr, context)
        } else {
            context.diagnostics.error(/* missing target type */);
            return;
        };

        // 2. Extract target symbol from Ty
        let target_symbol_id = match target_ty.kind() {
            TyKind::Struct { symbol, .. } => symbol.id(),
            TyKind::Protocol { symbol, .. } => symbol.id(),
            _ => {
                context.diagnostics.error(/* can only extend structs/protocols */);
                return;
            }
        };

        // 3. Add ExtensionTargetBehavior to extension
        extension.add_behavior(ExtensionTargetBehavior::new(target_ty.clone()));

        // 4. Resolve conformances (if any)
        if let Some(conformances) = extension_decl.conformance_list() {
            resolve_conformance_list(
                &conformances,
                extension.syntax(),
                source,
                &extension,
                context,
            );
        }

        // 5. Resolve where clause (if any)
        // Note: Type parameters come from target type
        // Inherit struct's constraints, add extension's additional constraints
        if let Some(where_clause) = extension_decl.where_clause() {
            resolve_where_clause_for_extension(
                &where_clause,
                &target_ty,
                extension,
                context,
            );
        }

        // 6. Register extension globally
        context.db.extension_registry.register(target_symbol_id, extension.id());

        // 7. Bind all method children
        for child in extension.metadata().children() {
            if let Some(func_sym) = child.downcast_ref::<FunctionSymbol>() {
                // Find function syntax node and bind it
                bind_function(func_sym, child_syntax, context);
            }
        }
    }
}
```

**Update:** `lib/kestrel-semantic-tree-builder/src/resolver.rs` (ResolverRegistry)

```rust
impl ResolverRegistry {
    pub fn get_resolver(&self, kind: SyntaxKind) -> Option<Arc<dyn Resolver>> {
        match kind {
            // ... existing cases
            SyntaxKind::ExtensionDeclaration => Some(Arc::new(ExtensionResolver)),
            // ...
        }
    }
}
```

---

### Phase 7: Type Parameter Scope Resolution

**Update:** `lib/kestrel-semantic-tree-builder/src/body_resolver/mod.rs`

Add a helper function to resolve names in extension context:

```rust
fn resolve_name_in_extension_context(
    name: &str,
    extension: &ExtensionSymbol,
    ctx: &BodyResolutionContext,
) -> Option<SymbolId> {
    // Steps 1-3: Check local scope, extension children, parent scope
    // (Already handled by existing resolution logic)

    // Step 4: NEW - Check target type's type parameters
    let target_behavior = extension.extension_target_behavior()?;
    let target_ty = target_behavior.target_ty();

    let target_symbol = match target_ty.kind() {
        TyKind::Struct { symbol, .. } => symbol,
        TyKind::Protocol { symbol, .. } => symbol,
        _ => return None,
    };

    // Get type parameters from target struct/protocol
    if let Some(generics) = target_symbol.generics_behavior() {
        for type_param in generics.type_parameters() {
            if type_param.metadata().name() == name {
                return Some(type_param.id());
            }
        }
    }

    None
}
```

Integrate this into the existing name resolution logic when resolving within an extension.

---

### Phase 8: Method Resolution with Extensions

**Update:** `lib/kestrel-semantic-tree-builder/src/body_resolver/utils.rs`

Modify method resolution to search extensions:

```rust
pub fn resolve_member_access_with_extensions(
    receiver_ty: &Ty,
    member_name: &str,
    ctx: &BodyResolutionContext,
) -> Result<SymbolId, ResolveError> {
    let container = get_type_container(receiver_ty, ctx)?;

    // 1. Search direct children (existing logic)
    if let Some(member) = search_children(container, member_name) {
        return Ok(member);
    }

    // 2. NEW: Search applicable extensions
    let target_symbol_id = container.id();
    let extensions = ctx.db.extension_registry.get_extensions_for(target_symbol_id);

    let mut candidates = Vec::new();

    for ext_id in extensions {
        let ext_symbol = ctx.db.get_symbol(ext_id)?;
        let extension = ext_symbol.downcast_ref::<ExtensionSymbol>()?;

        // Check if extension is applicable to this type
        if !is_extension_applicable(extension, receiver_ty, ctx) {
            continue;
        }

        // Search methods in this extension
        for child in extension.metadata().children() {
            if child.metadata().name() == member_name {
                let specificity = calculate_extension_specificity(extension);
                candidates.push((child.id(), specificity));
            }
        }
    }

    // 3. Handle conflicts (specialized wins)
    if candidates.is_empty() {
        return Err(ResolveError::MemberNotFound);
    }

    // Sort by specificity (highest first)
    candidates.sort_by_key(|(_, specificity)| std::cmp::Reverse(*specificity));

    // Check for ambiguity (same specificity)
    if candidates.len() > 1 && candidates[0].1 == candidates[1].1 {
        return Err(ResolveError::AmbiguousExtensionMethod);
    }

    Ok(candidates[0].0)
}

fn is_extension_applicable(
    extension: &ExtensionSymbol,
    instantiated_ty: &Ty,
    ctx: &BodyResolutionContext,
) -> bool {
    let target_behavior = match extension.extension_target_behavior() {
        Some(b) => b,
        None => return false,
    };
    let ext_target = target_behavior.target_ty();

    // Unify: Can instantiated_ty match ext_target pattern?
    if !types_unify(ext_target, instantiated_ty) {
        return false;
    }

    // Check where clause constraints
    check_where_clause_satisfied(extension, instantiated_ty, ctx)
}

fn calculate_extension_specificity(extension: &ExtensionSymbol) -> usize {
    // Count concrete type arguments in target
    let target_behavior = extension.extension_target_behavior().unwrap();
    let target_ty = target_behavior.target_ty();

    match target_ty.kind() {
        TyKind::Struct { substitutions: Some(subs), .. } |
        TyKind::Protocol { substitutions: Some(subs), .. } => {
            // Count how many substitutions are NOT type parameters
            subs.iter()
                .filter(|ty| !matches!(ty.kind(), TyKind::TypeParameter { .. }))
                .count()
        }
        _ => 0,
    }
}

fn types_unify(pattern: &Ty, concrete: &Ty) -> bool {
    match (pattern.kind(), concrete.kind()) {
        // Type parameter in pattern matches anything
        (TyKind::TypeParameter { .. }, _) => true,

        // Both structs: check symbol and recursively unify substitutions
        (
            TyKind::Struct { symbol: s1, substitutions: sub1 },
            TyKind::Struct { symbol: s2, substitutions: sub2 },
        ) => {
            if s1.id() != s2.id() {
                return false;
            }

            match (sub1, sub2) {
                (Some(subs1), Some(subs2)) => {
                    if subs1.len() != subs2.len() {
                        return false;
                    }
                    subs1.iter().zip(subs2.iter()).all(|(t1, t2)| types_unify(t1, t2))
                }
                (None, None) => true,
                _ => false,
            }
        }

        // Both same primitive type
        (TyKind::Int, TyKind::Int) |
        (TyKind::Float, TyKind::Float) |
        (TyKind::Bool, TyKind::Bool) |
        (TyKind::String, TyKind::String) => true,

        // Other combinations don't unify
        _ => false,
    }
}
```

---

### Phase 9: Validation

**New file:** `lib/kestrel-semantic-tree-builder/src/validation/extension.rs`

```rust
pub struct ExtensionValidator;

impl Validator for ExtensionValidator {
    fn finalize(&self, db: &SemanticDatabase, diagnostics: &mut DiagnosticContext) {
        // For each target type with extensions, check for conflicts
        for (target_id, ext_ids) in db.extension_registry.all_extensions() {
            validate_extension_conflicts(target_id, ext_ids, db, diagnostics);
        }
    }
}

fn validate_extension_conflicts(
    target_id: SymbolId,
    extension_ids: &[SymbolId],
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    // Collect all methods from all extensions with their signatures and specificity
    let mut methods: Vec<(MethodSignature, SymbolId, SymbolId, usize)> = Vec::new();

    for ext_id in extension_ids {
        let ext = db.get_symbol(ext_id).unwrap();
        let extension = ext.downcast_ref::<ExtensionSymbol>().unwrap();
        let specificity = calculate_extension_specificity(extension);

        for method in extension.metadata().children() {
            if let Some(func) = method.downcast_ref::<FunctionSymbol>() {
                let sig = get_method_signature(func);
                methods.push((sig, ext_id, method.id(), specificity));
            }
        }
    }

    // Check for conflicts: same signature, same specificity
    for i in 0..methods.len() {
        for j in (i+1)..methods.len() {
            let (sig1, ext1, method1, spec1) = &methods[i];
            let (sig2, ext2, method2, spec2) = &methods[j];

            if sig1 == sig2 && spec1 == spec2 {
                // Conflict! Same signature, same specificity
                let method1_sym = db.get_symbol(*method1).unwrap();
                let method2_sym = db.get_symbol(*method2).unwrap();
                let ext1_sym = db.get_symbol(*ext1).unwrap();
                let ext2_sym = db.get_symbol(*ext2).unwrap();

                diagnostics.error(DuplicateExtensionMethod {
                    method_name: sig1.name.clone(),
                    method1_span: method1_sym.metadata().span(),
                    method2_span: method2_sym.metadata().span(),
                    extension1_span: ext1_sym.metadata().span(),
                    extension2_span: ext2_sym.metadata().span(),
                });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MethodSignature {
    name: String,
    parameter_labels: Vec<Option<String>>,
    parameter_types: Vec<Ty>,
    return_type: Ty,
}

fn get_method_signature(func: &FunctionSymbol) -> MethodSignature {
    let callable = func.callable_behavior().unwrap();
    MethodSignature {
        name: func.metadata().name().to_string(),
        parameter_labels: callable.parameter_labels().to_vec(),
        parameter_types: callable.parameter_types().to_vec(),
        return_type: callable.return_type().clone(),
    }
}
```

**Update:** Register `ExtensionValidator` in the validation pipeline

```rust
// In validation/mod.rs or similar
pub fn run_validation(db: &SemanticDatabase) -> DiagnosticContext {
    let mut diagnostics = DiagnosticContext::new();

    // ... existing validators
    ConformanceValidator.finalize(db, &mut diagnostics);
    ExtensionValidator.finalize(db, &mut diagnostics);  // ADD THIS

    diagnostics
}
```

**Update:** `lib/kestrel-semantic-tree-builder/src/validation/conformance.rs`

Modify conformance checking to include extensions:

```rust
fn check_struct_conformance(
    struct_symbol: &StructSymbol,
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    // 1. Get struct's direct conformances (existing)
    let direct_conformances = struct_symbol.conformances_behavior()
        .map(|b| b.conformances())
        .unwrap_or(&[]);

    // 2. NEW: Get conformances from extensions
    let extensions = db.extension_registry.get_extensions_for(struct_symbol.id());
    let mut extension_conformances = Vec::new();

    for ext_id in extensions {
        let ext = db.get_symbol(ext_id).unwrap();
        if let Some(conf_behavior) = ext.conformances_behavior() {
            extension_conformances.extend(conf_behavior.conformances());
        }
    }

    // 3. Validate all conformances (both direct and from extensions)
    for conformance in direct_conformances.iter().chain(extension_conformances.iter()) {
        validate_protocol_conformance(
            struct_symbol,
            conformance,
            extensions,  // Pass extensions so we can find methods
            db,
            diagnostics,
        );
    }
}

fn validate_protocol_conformance(
    struct_symbol: &StructSymbol,
    protocol_ty: &Ty,
    extensions: &[SymbolId],
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    // ... existing logic to get required protocol methods

    // When searching for implementing methods, also search extensions:
    for required_method in required_methods {
        let mut found = false;

        // Search struct's direct children
        for child in struct_symbol.metadata().children() {
            if method_matches(child, required_method) {
                found = true;
                break;
            }
        }

        // NEW: Search extension methods
        if !found {
            for ext_id in extensions {
                let ext = db.get_symbol(ext_id).unwrap();
                for child in ext.metadata().children() {
                    if method_matches(child, required_method) {
                        found = true;
                        break;
                    }
                }
                if found { break; }
            }
        }

        if !found {
            diagnostics.error(MissingProtocolMethod { /* ... */ });
        }
    }
}
```

---

### Phase 10: Run Tests & Iterate

1. Run test suite: `cargo test extensions`
2. Fix failures one by one
3. Add more edge case tests as needed
4. Ensure all existing tests still pass

---

## Summary: Files to Create/Modify

### New Files (7)

1. `lib/kestrel-test-suite/tests/declarations/extensions.rs` - Comprehensive tests
2. `lib/kestrel-parser/src/extension/mod.rs` - Extension parser
3. `lib/kestrel-semantic-tree/src/symbol/extension.rs` - ExtensionSymbol
4. `lib/kestrel-semantic-tree/src/behavior/extension_target.rs` - ExtensionTargetBehavior
5. `lib/kestrel-semantic-tree/src/extension_registry.rs` - ExtensionRegistry
6. `lib/kestrel-semantic-tree-builder/src/resolvers/extension.rs` - ExtensionResolver
7. `lib/kestrel-semantic-tree-builder/src/validation/extension.rs` - ExtensionValidator

### Modified Files (10+)

1. `lib/kestrel-lexer/src/lib.rs` - Add `Extend` token
2. `lib/kestrel-syntax-tree/src/lib.rs` - Add ExtensionDeclaration, ExtensionBody nodes
3. `lib/kestrel-parser/src/declaration_item/mod.rs` - Route extension declarations
4. `lib/kestrel-semantic-tree/src/symbol/mod.rs` - Export ExtensionSymbol, add SymbolKind::Extension
5. `lib/kestrel-semantic-tree/src/behavior/mod.rs` - Export ExtensionTargetBehavior, add to BehaviorKind
6. `lib/kestrel-semantic-tree/src/lib.rs` - Export ExtensionRegistry
7. `lib/kestrel-semantic-tree/src/database.rs` - Add extension_registry field
8. `lib/kestrel-semantic-tree-builder/src/resolver.rs` - Register ExtensionResolver
9. `lib/kestrel-semantic-tree-builder/src/body_resolver/mod.rs` - Type parameter scope resolution
10. `lib/kestrel-semantic-tree-builder/src/body_resolver/utils.rs` - Method lookup with extensions
11. `lib/kestrel-semantic-tree-builder/src/validation/conformance.rs` - Check extension conformances
12. `lib/kestrel-semantic-tree-builder/src/validation/mod.rs` - Register ExtensionValidator

---

## Implementation Estimates

- **Phase 1 (Tests)**: 2-3 hours - Write comprehensive test suite
- **Phase 2 (Lexer/Syntax)**: 30 minutes - Add token and nodes
- **Phase 3 (Parser)**: 2-3 hours - Parse extension syntax
- **Phase 4 (Symbols)**: 1-2 hours - Create symbol and behaviors
- **Phase 5 (Registry)**: 1 hour - Extension tracking
- **Phase 6 (Resolver)**: 3-4 hours - BUILD and BIND logic
- **Phase 7 (Scope)**: 2 hours - Type parameter resolution
- **Phase 8 (Method Resolution)**: 3-4 hours - Lookup with unification
- **Phase 9 (Validation)**: 3-4 hours - Conflict detection and conformance checking
- **Phase 10 (Iterate)**: Variable - Fix bugs, refine

**Total Estimate**: 18-24 hours of focused implementation

---

## Success Criteria

✅ All tests in `extensions.rs` pass
✅ Can extend structs with methods
✅ Can add protocol conformances via extensions
✅ Generic extensions work (`extension Box[T]`)
✅ Specialized extensions work (`extension Box[Int]`)
✅ Specialized methods override generic ones
✅ Type parameters from struct are in scope
✅ Where clauses inherit and add constraints
✅ Conflict detection catches duplicate methods
✅ All existing tests still pass

---

## Future Enhancements (Not in Initial Implementation)

- Extension static variables
- Extending protocols (adding default implementations)
- Extending type aliases (desugar to underlying type)
- Extension-specific diagnostics and error messages
- IDE support (autocomplete extension methods)
- Documentation generation for extensions
