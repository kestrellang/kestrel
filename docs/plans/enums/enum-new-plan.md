# Enum Implementation Plan (Third Revision)

## Summary

Implement full enum support with parity to structs:
- Enum declarations with cases (simple and with associated values)
- Generic enums with type parameters and where clauses
- Protocol conformance (manual implementation only, no synthesized conformance)
- Methods (instance and static)
- Initializers
- Nested types (structs, enums inside enums)
- Extensions on enums (automatic once TyKind::Enum exists)
- `indirect` keyword for recursive enums
- Shorthand `.Case` syntax (enum cases only, not static members)

**Out of Scope:**
- Pattern matching / exhaustiveness checking (next major feature)
- Synthesized protocol conformances (Equatable, Hashable, etc.)

---

## Current Status

| Component | Status |
|-----------|--------|
| Lexer (`Enum`, `Case`, `Indirect` tokens) | **MISSING** |
| Syntax Tree (`SyntaxKind` variants) | **MISSING** |
| Parser (`enum_decl/mod.rs`) | **MISSING** |
| Semantic Symbols (`EnumSymbol`, `EnumCaseSymbol`) | **PARTIAL** - `KestrelSymbolKind` exists, no symbol files |
| Type System (`TyKind::Enum`) | **MISSING** |
| Builders | **MISSING** |
| Binders | **MISSING** |
| Body Resolver | Clean - no `expected_type` field |
| Expression Types (`ExprKind::ImplicitMemberAccess`) | **MISSING** |
| Type Inference Constraints | **MISSING** |
| Analyzers | **MISSING** |
| Tests | **EXISTS** - comprehensive test suite (~1140 lines) |

---

## Implementation Phases

### Phase 1: Lexer Tokens
**File:** `lib/kestrel-lexer/src/lib.rs`

Add tokens:
```rust
#[token("enum")]
Enum,

#[token("case")]
Case,

#[token("indirect")]
Indirect,
```

All three are reserved keywords.

---

### Phase 2: Syntax Tree Nodes
**File:** `lib/kestrel-syntax-tree/src/lib.rs`

Add to `SyntaxKind` enum:
```rust
// Enum declaration nodes
EnumDeclaration,
EnumBody,
EnumCaseDeclaration,
EnumCaseParameter,
EnumCaseParameterList,
IndirectModifier,

// Enum keyword tokens
Enum,
Case,
Indirect,

// Expression node
ExprImplicitMemberAccess,
```

Update mappings:
- `Token::Enum → SyntaxKind::Enum`
- `Token::Case → SyntaxKind::Case`
- `Token::Indirect → SyntaxKind::Indirect`
- Add `kind_from_raw` match arms for all new variants

---

### Phase 3: Parser Data Structures
**File:** `lib/kestrel-parser/src/common/data.rs`

#### 3.1 Unified Type Declaration Body Item

To enable mutual recursion between struct and enum parsers for nested types, introduce a unified body item type:

```rust
/// Items that can appear in a type declaration body (struct or enum)
/// Used to enable mutual nesting of structs and enums
#[derive(Debug, Clone)]
pub enum TypeDeclarationBodyItem {
    Field(FieldDeclarationData),
    Function(FunctionDeclarationData),
    Initializer(InitializerDeclarationData),
    Struct(Box<StructDeclarationData>),    // Boxed to avoid infinite size
    Enum(Box<EnumDeclarationData>),        // Boxed to avoid infinite size
    EnumCase(EnumCaseDeclarationData),     // Only valid in enum bodies
    TypeAlias(TypeAliasDeclarationData),
    Module(Span, Vec<Span>),
    Import(Span, Vec<Span>, Option<Span>, Option<Vec<(Span, Option<Span>)>>),
}
```

#### 3.2 Enum-Specific Data Structures

```rust
/// Raw parsed data for enum case parameter (label: Type)
#[derive(Debug, Clone)]
pub struct EnumCaseParameterData {
    pub label: Span,
    pub colon: Span,
    pub ty: TyVariant,
}

/// Raw parsed data for enum case declaration
#[derive(Debug, Clone)]
pub struct EnumCaseDeclarationData {
    pub case_span: Span,
    pub name_span: Span,
    pub parameters: Option<(Span, Vec<EnumCaseParameterData>, Span)>, // (lparen, params, rparen)
}

/// Raw parsed data for enum declaration
#[derive(Debug, Clone)]
pub struct EnumDeclarationData {
    pub visibility: Option<(Token, Span)>,
    pub indirect: Option<Span>,
    pub enum_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub conformances: Option<ConformanceListData>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<TypeDeclarationBodyItem>,
    pub rbrace_span: Span,
}
```

#### 3.3 Update StructDeclarationData

Update to use the unified body item type:

```rust
pub struct StructDeclarationData {
    // ... existing fields ...
    pub body: Vec<TypeDeclarationBodyItem>,  // Changed from Vec<StructBodyItem>
}
```

**Migration note:** The old `StructBodyItem` enum can be removed. Builders will filter `TypeDeclarationBodyItem` variants as appropriate (e.g., reject `EnumCase` in struct bodies).

---

### Phase 4: Unified Type Declaration Parser

**New file:** `lib/kestrel-parser/src/type_declaration/mod.rs`

The key insight is to use a **single `recursive()` call** with a wrapper enum to enable mutual nesting:

```rust
/// Wrapper enum for unified type declaration parsing
#[derive(Debug, Clone)]
pub enum TypeDeclarationData {
    Struct(StructDeclarationData),
    Enum(EnumDeclarationData),
}

pub fn type_declaration_parser_internal()
-> impl Parser<Token, TypeDeclarationData, Error = Simple<Token>> + Clone {
    recursive(|type_decl_parser| {
        // Body item parser that accepts the unified recursive handle
        let body_item_parser = type_body_item_parser_internal(type_decl_parser.clone());
        
        // Struct parser using shared body items
        let struct_parser = struct_parser_with_body(body_item_parser.clone())
            .map(TypeDeclarationData::Struct);
        
        // Enum parser using shared body items
        let enum_parser = enum_parser_with_body(body_item_parser)
            .map(TypeDeclarationData::Enum);
        
        struct_parser.or(enum_parser)
    })
}

fn type_body_item_parser_internal(
    type_decl_parser: impl Parser<Token, TypeDeclarationData, Error = Simple<Token>> + Clone,
) -> impl Parser<Token, TypeDeclarationBodyItem, Error = Simple<Token>> + Clone {
    // Convert nested type declarations to body items
    let nested_type_parser = type_decl_parser.map(|data| match data {
        TypeDeclarationData::Struct(s) => TypeDeclarationBodyItem::Struct(Box::new(s)),
        TypeDeclarationData::Enum(e) => TypeDeclarationBodyItem::Enum(Box::new(e)),
    });
    
    let case_parser = enum_case_parser().map(TypeDeclarationBodyItem::EnumCase);
    let function_parser = function_declaration_parser_internal().map(TypeDeclarationBodyItem::Function);
    let initializer_parser = initializer_declaration_parser_internal().map(TypeDeclarationBodyItem::Initializer);
    let type_alias_parser = type_alias_declaration_parser_internal().map(TypeDeclarationBodyItem::TypeAlias);
    let field_parser = field_declaration_parser_internal().map(TypeDeclarationBodyItem::Field);
    // ... module, import parsers ...
    
    nested_type_parser
        .or(case_parser)
        .or(initializer_parser)
        .or(function_parser)
        .or(type_alias_parser)
        .or(field_parser)
        // ... etc
}
```

#### 4.1 Enum-Specific Parsers

```rust
fn indirect_modifier_parser() -> impl Parser<Token, Option<Span>, Error = Simple<Token>> + Clone {
    just(Token::Indirect)
        .map_with_span(|_, span| Some(span))
        .or(empty().map(|_| None))
}

fn enum_case_parameter_parser() -> impl Parser<Token, EnumCaseParameterData, Error = Simple<Token>> + Clone {
    identifier()
        .then_ignore(just(Token::Colon))
        .then(ty_parser())
        .map_with_span(|(label, ty), span| EnumCaseParameterData { label, colon: span, ty })
}

fn enum_case_parser() -> impl Parser<Token, EnumCaseDeclarationData, Error = Simple<Token>> + Clone {
    just(Token::Case)
        .map_with_span(|_, span| span)
        .then(identifier())
        .then(
            enum_case_parameter_parser()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .delimited_by(just(Token::LParen), just(Token::RParen))
                .map_with_span(|params, span| Some((span, params)))
                .or(empty().map(|_| None))
        )
        .map(|((case_span, name_span), parameters)| EnumCaseDeclarationData {
            case_span,
            name_span,
            parameters,
        })
}
```

#### 4.2 Error Recovery Strategy

For malformed enum declarations, use these recovery points:

1. **Missing case name:** Skip to next `case` keyword or `}`
2. **Malformed parameter list:** Skip to `)` or next `case`/`}`
3. **Missing closing brace:** Recover at next top-level declaration

```rust
fn enum_body_with_recovery(
    body_item_parser: impl Parser<Token, TypeDeclarationBodyItem, ...>
) -> impl Parser<...> {
    body_item_parser
        .recover_with(skip_then_retry_until([Token::Case, Token::Func, Token::RBrace]))
        .repeated()
}
```

**File updates:**
- `lib/kestrel-parser/src/declaration_item/mod.rs` - Route through unified type declaration parser
- `lib/kestrel-parser/src/struct/mod.rs` - Refactor to use shared body item parser
- `lib/kestrel-parser/src/lib.rs` - Add `pub mod type_declaration;`

**File:** `lib/kestrel-parser/src/common/emitters.rs`

Add emitter functions:
- `emit_enum_declaration`
- `emit_enum_case`
- `emit_enum_case_parameter`
- `emit_enum_case_parameter_list`
- `emit_indirect_modifier`

---

### Phase 5: Implicit Member Access Expression Parser
**File:** `lib/kestrel-parser/src/expr/mod.rs`

Add to primary expression alternatives:
```rust
// .Case or .Case(args) - implicit member access for enum cases ONLY
fn implicit_member_access_parser() -> impl Parser<...> {
    just(Token::Dot)
        .ignore_then(identifier())
        .then(argument_list_parser().or_not())
        .map_with_span(|(name, args), span| ImplicitMemberAccessData { 
            dot_span: span,
            name,
            arguments: args,
        })
}
```

Emit as `ExprImplicitMemberAccess` node containing:
- `Dot` token
- `Name` node
- Optional `ArgumentList` node

**Design Decision:** `.foo` syntax is exclusively for enum cases. If the expected type is not an enum, emit error E0403 ("cannot infer enum type for shorthand"). This prevents confusion with potential future static member access syntax.

---

### Phase 6: Semantic Symbols
**New file:** `lib/kestrel-semantic-tree/src/symbol/enum_symbol.rs`

```rust
pub struct EnumSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
    is_indirect: bool,
}

impl Symbol<KestrelLanguage> for EnumSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl EnumSymbol {
    pub fn new(name: Name, span: Span, visibility: VisibilityBehavior, 
               is_indirect: bool, parent: Option<Arc<dyn Symbol<KestrelLanguage>>>) -> Self;
    
    pub fn is_indirect(&self) -> bool;
    pub fn cases(&self) -> Vec<Arc<EnumCaseSymbol>>;
    pub fn type_parameters(&self) -> Vec<Arc<TypeParameterSymbol>>;
    pub fn type_parameter_count(&self) -> usize;
    pub fn is_generic(&self) -> bool;
    pub fn where_clause(&self) -> WhereClause;
}
```

**New file:** `lib/kestrel-semantic-tree/src/symbol/enum_case.rs`

```rust
pub struct EnumCaseSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl Symbol<KestrelLanguage> for EnumCaseSymbol {
    fn metadata(&self) -> &SymbolMetadata<KestrelLanguage> {
        &self.metadata
    }
}

impl EnumCaseSymbol {
    pub fn new(name: Name, span: Span, parent: Arc<dyn Symbol<KestrelLanguage>>) -> Self;
    
    /// Returns true if this case has associated values (has CallableBehavior)
    pub fn has_associated_values(&self) -> bool {
        self.metadata.get_behavior::<CallableBehavior>().is_some()
    }
}
```

**Design Decision:** Cases with empty parameter lists `case Foo()` are treated the same as `case Foo` - no `CallableBehavior` is attached. The syntax `Foo()` is still valid but semantically identical to `Foo`.

**Update:** `lib/kestrel-semantic-tree/src/symbol/mod.rs`

Export new symbols:
```rust
pub mod enum_symbol;
pub mod enum_case;
pub use enum_symbol::EnumSymbol;
pub use enum_case::EnumCaseSymbol;
```

---

### Phase 7: Type System
**File:** `lib/kestrel-semantic-tree/src/ty/kind.rs`

Add variant:
```rust
/// An enum type with substitutions for generic parameters
Enum {
    symbol: Arc<EnumSymbol>,
    substitutions: Substitutions,
},
```

**Design Decision:** No `TyKind::ImplicitMember` variant. The expression `ExprKind::ImplicitMemberAccess` has type `Ty::infer()`, and the constraint solver resolves it when the expected type becomes known through unification.

**File:** `lib/kestrel-semantic-tree/src/ty/mod.rs`

Add constructor:
```rust
impl Ty {
    pub fn enum_type(symbol: Arc<EnumSymbol>, span: Span) -> Self {
        Self::new(TyKind::Enum { 
            symbol, 
            substitutions: Substitutions::new() 
        }, span)
    }
    
    pub fn generic_enum(symbol: Arc<EnumSymbol>, substitutions: Substitutions, span: Span) -> Self {
        Self::new(TyKind::Enum { symbol, substitutions }, span)
    }
}
```

#### 7.1 Generic Enum Instantiation

When resolving a type expression like `Result<Int, String>`:

1. Type resolver encounters `TyPath` with type arguments
2. Looks up `Result` → finds `EnumSymbol` with type params `[T, E]`
3. Creates substitutions: `{T.id → Int, E.id → String}`
4. Returns `Ty::generic_enum(result_symbol, substitutions, span)`

This follows the existing pattern for `TyKind::Struct`. See `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs:apply_type_arguments`.

---

### Phase 8: Expression Types
**File:** `lib/kestrel-semantic-tree/src/expr.rs`

Add variants to `ExprKind`:
```rust
/// Reference to a resolved enum case (simple case without arguments)
EnumCase {
    case_id: SymbolId,
},

/// Unresolved implicit member access: .Case or .Case(args)
/// Type inference resolves this to EnumCase or validates against expected type
ImplicitMemberAccess {
    member_name: String,
    arguments: Option<Vec<CallArgument>>,
},
```

Add constructor methods to `Expression`:
```rust
impl Expression {
    pub fn enum_case(case_id: SymbolId, ty: Ty, span: Span) -> Self;
    
    /// Creates an implicit member access with Ty::infer()
    /// Type inference will resolve the actual type
    pub fn implicit_member_access(member_name: String, arguments: Option<Vec<CallArgument>>, span: Span) -> Self {
        Self::new(
            ExprKind::ImplicitMemberAccess { member_name, arguments },
            Ty::infer(span.clone()),
            span,
        )
    }
}
```

---

### Phase 9: Builder
**New file:** `lib/kestrel-semantic-tree-builder/src/builders/enum.rs`

```rust
pub fn build_enum(
    node: &SyntaxNode,
    source: &str,
    file_id: usize,
    parent: Option<Arc<dyn Symbol<KestrelLanguage>>>,
    model: &mut SemanticModel,
) -> Arc<EnumSymbol>
```

Responsibilities:
- Extract name, span, visibility from syntax
- Detect `IndirectModifier` node
- Build child type parameters
- Build child cases, functions, initializers, nested types
- Register symbol in model
- **Validate body items:** Reject `Field` variants (fields not allowed in enums)

**New file:** `lib/kestrel-semantic-tree-builder/src/builders/enum_case.rs`

```rust
pub fn build_enum_case(
    node: &SyntaxNode,
    source: &str,
    file_id: usize,
    parent: Arc<dyn Symbol<KestrelLanguage>>,
    model: &mut SemanticModel,
) -> Arc<EnumCaseSymbol>
```

**Update:** `lib/kestrel-semantic-tree-builder/src/builders/struct.rs`

Update struct builder to:
- Use `TypeDeclarationBodyItem` instead of `StructBodyItem`
- Handle `Enum` variants for nested enums
- Reject `EnumCase` variants with appropriate error

**Update:** `lib/kestrel-semantic-tree-builder/src/builders/mod.rs`

- Add enum building dispatch in declaration item handling

---

### Phase 10: Binder
**New file:** `lib/kestrel-semantic-tree-binder/src/binders/enum.rs`

```rust
pub struct EnumBinder;

impl Binder for EnumBinder {
    fn bind(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut BindingContext) {
        // 1. Resolve type parameters
        // 2. Resolve where clause
        // 3. Attach GenericsBehavior
        // 4. Resolve conformances (protocol conformance)
    }
}
```

**New file:** `lib/kestrel-semantic-tree-binder/src/binders/enum_case.rs`

```rust
pub struct EnumCaseBinder;

impl Binder for EnumCaseBinder {
    fn bind(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut BindingContext) {
        // 1. Resolve parameter types (using parent enum's generic scope)
        // 2. Check for duplicate labels → emit E0406
        // 3. Attach CallableBehavior ONLY if case has non-empty parameters
        //    - case Foo → no CallableBehavior
        //    - case Foo() → no CallableBehavior (empty params = no callable)
        //    - case Foo(x: Int) → CallableBehavior with params
    }
}
```

**Update:** `lib/kestrel-semantic-tree-binder/src/binders/mod.rs`

Register new binders.

---

### Phase 11: Body Resolver
**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

Add handler for `SyntaxKind::ExprImplicitMemberAccess`:

```rust
fn resolve_implicit_member_access(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);
    
    // Extract member name from Name child
    let member_name = node
        .children()
        .find(|c| c.kind() == SyntaxKind::Name)
        .and_then(|n| extract_identifier_from_name(&n))
        .unwrap_or_else(|| "?".to_string());
    
    // Extract arguments if ArgumentList present
    let arguments = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ArgumentList)
        .map(|arg_list| resolve_argument_list(&arg_list, ctx));
    
    // Return with Ty::infer() - type inference will resolve via constraints
    Expression::implicit_member_access(member_name, arguments, span)
}
```

**Important:** Body resolver creates `Expression` with `Ty::infer()`. The expected type flows through constraints.

---

### Phase 12: Type Inference - Constraints
**File:** `lib/kestrel-semantic-type-inference/src/constraint.rs`

Add constraint variant:
```rust
/// Resolve an implicit member access when expected type becomes known
ImplicitMember {
    /// The expression's type variable (starts as Infer)
    expr_ty: TyId,
    /// The member name (case name)
    member_name: String,
    /// Argument types if provided (already have their own constraints)
    argument_tys: Vec<(Option<String>, TyId)>,  // (label, type_id)
    /// Expression ID for value resolution recording
    expr_id: ExprId,
    /// Span for error reporting
    span: Span,
},
```

**File:** `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

When visiting `ExprKind::ImplicitMemberAccess`:
```rust
ExprKind::ImplicitMemberAccess { member_name, arguments } => {
    // Register expression type as inference variable
    ctx.register_type(&expr.ty);
    
    // Process arguments and register their types
    let argument_tys = arguments.as_ref().map(|args| {
        args.iter().map(|arg| {
            generate_expression_constraints(ctx, &arg.value);
            ctx.register_type(&arg.value.ty);
            (arg.label.clone(), arg.value.ty.id())
        }).collect()
    }).unwrap_or_default();
    
    // Create constraint - will be solved when expr_ty is unified with expected type
    ctx.add_constraint(Constraint::ImplicitMember {
        expr_ty: expr.ty.id(),
        member_name: member_name.clone(),
        argument_tys,
        expr_id: expr.id,
        span: expr.span.clone(),
    });
}
```

#### How Expected Type Flows

The key mechanism is the existing `Constraint::Equals` generation:

```rust
// In let x: SomeType = expr
// constraint_generator.rs generates:
ctx.equate(pattern.ty.id(), expr.ty.id(), stmt.span.clone());
```

When `pattern.ty` is `Option<Int>` and `expr.ty` is `Infer` (from `.None`):
1. Solver unifies them → `expr.ty` becomes `Option<Int>`
2. `ImplicitMember` constraint now sees resolved type
3. Constraint solver validates `.None` against `Option<Int>`

This works because constraint generation for bindings, function arguments, and returns already creates these equate constraints.

---

### Phase 13: Type Inference - Solver
**File:** `lib/kestrel-semantic-type-inference/src/solver.rs`

#### 13.1 Enum Unification

Add case in `unify()` function:
```rust
(
    TyKind::Enum { symbol: sym_a, substitutions: subs_a },
    TyKind::Enum { symbol: sym_b, substitutions: subs_b },
) => {
    // Must be same enum
    let id_a = sym_a.metadata().id();
    let id_b = sym_b.metadata().id();
    
    if id_a != id_b {
        return Err(InferenceError::type_mismatch(ty_a.clone(), ty_b.clone(), span.clone()));
    }
    
    // Unify substitutions
    for (key, sub_a) in subs_a.iter() {
        if let Some(sub_b) = subs_b.get(*key) {
            ctx.equate(sub_a.id(), sub_b.id(), span.clone());
        }
    }
    
    Ok(SolveResult::Solved)
}
```

#### 13.2 ImplicitMember Constraint Solver

Add handler in `try_solve`:
```rust
Constraint::ImplicitMember { expr_ty, member_name, argument_tys, expr_id, span } => {
    resolve_implicit_member(ctx, *expr_ty, member_name, argument_tys, *expr_id, span)
}
```

Implementation:
```rust
fn resolve_implicit_member(
    ctx: &mut InferenceContext<'_>,
    expr_ty: TyId,
    member_name: &str,
    argument_tys: &[(Option<String>, TyId)],
    expr_id: ExprId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let resolved_ty = resolve_type(ctx, expr_ty);
    
    // If still Infer, defer until more info available
    if matches!(resolved_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }
    
    // Must be an enum type - .Case syntax is ONLY for enums
    let TyKind::Enum { symbol: enum_symbol, substitutions } = resolved_ty.kind() else {
        return Err(InferenceError::cannot_infer_enum_type(span.clone())); // E0403
    };
    
    // Look up the case by name
    let cases = enum_symbol.cases();
    let case = cases.iter().find(|c| c.metadata().name().value == member_name);
    
    let Some(case) = case else {
        let available = cases.iter().map(|c| c.metadata().name().value.clone()).collect();
        return Err(InferenceError::unknown_enum_case(
            member_name.to_string(),
            enum_symbol.metadata().name().value.clone(),
            available,
            span.clone(),
        )); // E0401
    };
    
    // Get callable behavior if case has parameters
    let callable = case.metadata().get_behavior::<CallableBehavior>();
    
    match (callable, argument_tys.is_empty()) {
        // Simple case, no args expected, none provided
        (None, true) => {
            ctx.values_mut().insert(expr_id, ValueResolution::simple(case.metadata().id()));
            Ok(SolveResult::Solved)
        }
        
        // Simple case but args provided - error
        (None, false) => {
            Err(InferenceError::enum_case_arity(
                member_name.to_string(), 0, argument_tys.len(), span.clone()
            )) // E0408
        }
        
        // Case with params, no args provided
        (Some(cb), true) if !cb.parameters().is_empty() => {
            Err(InferenceError::enum_case_arity(
                member_name.to_string(), cb.parameters().len(), 0, span.clone()
            )) // E0408
        }
        
        // Case with params, args provided - validate
        (Some(cb), _) => {
            let params = cb.parameters();
            
            // Check arity
            if params.len() != argument_tys.len() {
                return Err(InferenceError::enum_case_arity(
                    member_name.to_string(), params.len(), argument_tys.len(), span.clone()
                )); // E0408
            }
            
            // Check labels and types
            for (i, ((label, arg_ty_id), param)) in argument_tys.iter().zip(params.iter()).enumerate() {
                let expected_label = param.label.as_ref().map(|l| l.value.as_str());
                let actual_label = label.as_deref();
                
                if actual_label != expected_label {
                    return Err(InferenceError::enum_case_label_mismatch(
                        member_name.to_string(),
                        expected_label.map(String::from),
                        label.clone(),
                        i,
                        span.clone(),
                    )); // E0402
                }
                
                // Equate argument type with parameter type (applying substitutions)
                let param_ty = param.ty.apply_substitutions(substitutions);
                ctx.equate(*arg_ty_id, param_ty.id(), span.clone());
            }
            
            ctx.values_mut().insert(expr_id, ValueResolution::simple(case.metadata().id()));
            Ok(SolveResult::Solved)
        }
    }
}
```

**Note on ValueResolution:** The existing `ValueResolution::simple(symbol_id)` is sufficient. No new variants needed - the symbol ID points to an `EnumCaseSymbol`, and downstream code can look it up to determine it's an enum case.

---

### Phase 14: Analyzers
**New file:** `lib/kestrel-semantic-analyzers/src/analyzers/enum_validation/mod.rs`

#### 14.1 Recursive Enum Analyzer

```rust
pub struct RecursiveEnumAnalyzer;

impl Analyzer for RecursiveEnumAnalyzer {
    fn analyze(&mut self, model: &SemanticModel, ctx: &mut AnalysisContext) {
        for enum_symbol in model.symbols_of_kind::<EnumSymbol>() {
            if !enum_symbol.is_indirect() && is_recursive(enum_symbol, model) {
                ctx.emit_error(E0404, enum_symbol.metadata().span());
            }
        }
    }
}

fn is_recursive(enum_symbol: &EnumSymbol, model: &SemanticModel) -> bool {
    // Check if any case parameter type contains a reference to this enum
    // Must check through:
    // - Direct reference: case Node(child: Tree)
    // - Generic wrapper: case Node(child: Box<Tree>) - still infinite without indirect
    // - Transitive: Tree -> Forest -> Tree
    
    // Use a visited set to prevent infinite loops
    let mut visited = HashSet::new();
    check_types_for_recursion(enum_symbol, &enum_symbol.cases(), &mut visited, model)
}
```

**Important:** This analysis runs AFTER building and binding, so types are fully resolved. The builder is lazy and won't infinite-loop during construction.

#### 14.2 Duplicate Case Analyzer

```rust
pub struct DuplicateCaseAnalyzer;

impl Analyzer for DuplicateCaseAnalyzer {
    fn analyze(&mut self, model: &SemanticModel, ctx: &mut AnalysisContext) {
        for enum_symbol in model.symbols_of_kind::<EnumSymbol>() {
            let mut seen: HashMap<&str, Span> = HashMap::new();
            for case in enum_symbol.cases() {
                let name = &case.metadata().name().value;
                if let Some(first_span) = seen.get(name.as_str()) {
                    ctx.emit_error(E0405, case.metadata().span(), first_span.clone());
                } else {
                    seen.insert(name, case.metadata().span());
                }
            }
        }
    }
}
```

**New file:** `lib/kestrel-semantic-analyzers/src/analyzers/enum_validation/diagnostics.rs`

Define diagnostic messages for E0404, E0405.

**Update:** `lib/kestrel-semantic-analyzers/src/analyzers/mod.rs`

Register new analyzers.

---

### Phase 15: Protocol Conformance

Enum protocol conformance works like struct conformance:
- Enums can declare conformances: `enum Foo: SomeProtocol { ... }`
- Protocol methods must be manually implemented in the enum body
- No synthesized conformances (Equatable, Hashable, etc.)

**How to implement protocol methods on enums:**

```kestrel
protocol Describable {
    func describe() -> String
}

enum Color: Describable {
    case Red
    case Green
    case Blue
    
    func describe() -> String {
        // Implementation requires pattern matching (Phase 2 work)
        // For now, can only return constant or use other means
        "a color"
    }
}
```

**Note:** Full protocol method implementation on enums requires pattern matching on `self`, which is out of scope. Initial implementation allows declaring conformance and implementing methods that don't need to switch on `self`.

---

### Phase 16: Error Codes Summary

| Code | Error | Emitted By |
|------|-------|------------|
| E0401 | Unknown enum case | Solver (ImplicitMember) |
| E0402 | Missing/wrong associated value label | Solver (ImplicitMember) |
| E0403 | Cannot infer enum type for shorthand | Solver (ImplicitMember) |
| E0404 | Recursive enum requires `indirect` | Analyzer |
| E0405 | Duplicate case name | Analyzer |
| E0406 | Duplicate label in case parameters | Binder (EnumCaseBinder) |
| E0407 | Associated value type mismatch | Solver (unification) |
| E0408 | Wrong number of associated values | Solver (ImplicitMember) |

---

## Implementation Order

| Step | Task | Depends On | Complexity |
|------|------|------------|------------|
| 1 | Lexer tokens (`Enum`, `Case`, `Indirect`) | - | Low |
| 2 | Syntax tree nodes and mappings | Step 1 | Low |
| 3 | Parser data structures (unified body item) | - | Medium |
| 4 | Unified type declaration parser | Steps 1-3 | Medium-High |
| 5 | Implicit member access parser | Steps 1-2 | Low |
| 6 | `EnumSymbol`, `EnumCaseSymbol` | - | Medium |
| 7 | `TyKind::Enum` | Step 6 | Low |
| 8 | `ExprKind::EnumCase`, `ExprKind::ImplicitMemberAccess` | - | Low |
| 9 | Enum builder | Steps 4, 6 | Medium |
| 10 | Enum binder | Steps 6, 9 | Medium |
| 11 | Body resolver for implicit member access | Steps 5, 8 | Low |
| 12 | `ImplicitMember` constraint | Steps 7, 8 | Medium |
| 13 | Enum unification in solver | Step 7 | Low |
| 14 | ImplicitMember constraint solver | Steps 10, 12, 13 | High |
| 15 | Analyzers (recursive, duplicate) | Steps 6, 9, 10 | Medium |
| 16 | Run tests, fix issues | All | Variable |

---

## Test Updates Required

The following test expects `indirect` to be a valid identifier, but it's now a reserved keyword:

```rust
// lib/kestrel-test-suite/tests/declarations/enums.rs
#[test]
fn indirect_keyword_as_identifier_in_different_context() {
    // This test should be REMOVED
    // `indirect` is now a reserved keyword
}
```

---

## Notes

1. **Extensions on enums** work automatically once `TyKind::Enum` exists, since extensions target any type expression.

2. **Enum methods** are parsed like struct methods - `FunctionDeclarationData` in the body.

3. **Protocol conformance** uses the same `ConformanceListData` as structs.

4. **`indirect`** is a reserved keyword (not contextual).

5. **`case`** is a reserved keyword.

6. **`.Case` syntax** is exclusively for enum cases. Attempting to use it when the expected type is not an enum produces error E0403.

7. **`case Foo()` vs `case Foo`**: Both are semantically equivalent (no associated values). No `CallableBehavior` is attached to either.

8. **Pattern matching** is the next major feature after this implementation. It will enable:
   - `switch` expressions on enum values
   - Exhaustiveness checking
   - Extracting associated values

9. **Recursion detection** happens during analysis, not building. The builder is lazy and type references don't cause infinite loops during symbol construction.
