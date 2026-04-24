# Component Reference

Components are attached to declaration entities during the build phase. They describe capabilities and metadata orthogonally.

## Identity

### `NodeKind`
**Data**: enum variant
**On**: every declaration entity
**Purpose**: Identifies what kind of declaration this entity represents.

Variants: `Module`, `Struct`, `Enum`, `EnumCase`, `Protocol`, `Extension`, `Function`, `Initializer`, `Deinit`, `Field`, `Subscript`, `TypeAlias`, `Import`, `TypeParameter`

### `DeclSpan`
**Data**: `Span` (file_id, start, end)
**On**: every declaration entity
**Purpose**: Source span excluding leading trivia. Used for error reporting and goto-definition.

### `CstNode`
**Data**: `SyntaxNode` (Arc-backed)
**On**: every declaration entity
**Purpose**: Cheap reference back to the CST for later phases that need expression details.

## Naming & Location

### `Name`
**Data**: `String`
**On**: all except Extension, Deinit
**Purpose**: The declared identifier name.

### `FileId`
**Data**: `Entity` (file entity handle)
**On**: all except Module
**Purpose**: Which source file this declaration came from. Modules span multiple files so they don't get FileId.

### `Vis`
**Data**: enum (`Public`, `Private`, `Internal`, `Fileprivate`)
**On**: declarations with visibility modifiers
**Purpose**: Access control for name resolution.

## Capability Components

### `Typed`
**Marker** (no data)
**On**: Struct, Enum, Protocol, TypeAlias
**Answers**: "Can this entity appear in type positions?" (e.g., `var x: ThisType`)

### `TypeAnnotation`
**Data**: `AstType`
**On**: Field (type), Function (return type), TypeAlias (target type), Subscript (return type), TypeParameter (default type)
**Answers**: "What type is declared here?"

### `Callable`
**Data**: `{ params: Vec<AstParam>, receiver: Option<ReceiverKind> }`
**On**: Function, Initializer, EnumCase (with associated values), Subscript
**Answers**: "What parameters does this accept? Does it have a self receiver?"

### `Gettable`
**Marker** (no data)
**On**: Field, Subscript
**Answers**: "Can this be read as a value?"

### `Settable`
**Marker** (no data)
**On**: `var` fields, computed properties with setter, subscripts with setter
**Answers**: "Can this be written to?"

### `Valued`
**Data**: `SyntaxNode` (CST subtree)
**On**: Function (body), Initializer (body), Deinit (body), Field (default value or getter body)
**Answers**: "Does this have executable code?"

### `Static`
**Marker** (no data)
**On**: Functions, Fields, Subscripts with `static` modifier
**Answers**: "Is this accessed through the type rather than an instance?"

### `Subscript`
**Marker** (no data)
**On**: Subscript declarations
**Answers**: "Is this accessed via call syntax on the parent (`obj(key)`)?"

## Generics

### `TypeParams`
**Data**: `Vec<Entity>` (TypeParameter entity handles)
**On**: Struct, Enum, Protocol, Function, Initializer, Subscript, TypeAlias
**Answers**: "What type parameters does this declare?"

### `WhereClause`
**Data**: `Vec<WhereConstraint>`
**On**: entities with type parameters
**Answers**: "What constraints apply to the type parameters?"

WhereConstraint variants:
- `Bound { subject, protocols, node }` — `T: Comparable`
- `Equality { lhs, rhs, node }` — `T.Item == Int`
- `NegativeBound { subject, protocol, node }` — `T: not Copyable`

## Type Relations

### `Conformances`
**Data**: `Vec<ConformanceItem>`
**On**: Struct, Enum, Protocol, Extension
**Answers**: "What protocols does this type conform to?"

ConformanceItem variants:
- `Positive(AstType, SyntaxNode)` — conformance
- `Negative(AstType, SyntaxNode)` — negative conformance

### `ExtensionTarget`
**Data**: `AstType`
**On**: Extension
**Answers**: "What type is being extended?"

## Modifiers

### `IsIndirect`
**Marker** (no data)
**On**: Enum
**Answers**: "Does this enum use indirect (heap-allocated) representation?"

## Metadata

### `Attributes`
**Data**: `Vec<AstAttribute>`
**On**: any declaration with `@attribute` annotations
**Answers**: "What attributes are applied?"

### `Documentation`
**Data**: `String`
**On**: any declaration with `///` doc comments
**Answers**: "What documentation was written?"

## Import-Specific

### `ModulePath`
**Data**: `Vec<String>` (path segments)
**On**: Import
**Answers**: "What module is being imported?"

### `ImportAlias`
**Data**: `String`
**On**: Import with `as` clause
**Answers**: "What alias is used for the import?"

### `ImportItems`
**Data**: `Vec<ImportItem>` (name + optional alias)
**On**: Import with selective items
**Answers**: "What specific items are imported?"
