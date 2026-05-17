# Entity-Component Mapping

Quick reference: for each declaration kind, what components does it get?

`[Brackets]` = optional, set only when present in source.

## Module
| Component | Notes |
|-----------|-------|
| `NodeKind::Module` | always |
| `Name` | module segment name |

No FileId — modules span multiple files.

## Struct
| Component | Notes |
|-----------|-------|
| `NodeKind::Struct` | always |
| `Name` | struct name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Typed` | always (marker) |
| `Conformances` | if `: Protocol` list present |
| `TypeParams` | if `[T, U]` present |
| `WhereClause` | if `where` clause present |
| `Attributes` | if `@attr` present |
| `Documentation` | if `///` comments present |

## Enum
| Component | Notes |
|-----------|-------|
| `NodeKind::Enum` | always |
| `Name` | enum name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Typed` | always (marker) |
| `IsIndirect` | if `indirect` modifier |
| `Conformances` | if conformance list |
| `TypeParams` | if type parameters |
| `WhereClause` | if where clause |
| `Attributes` | if attributes |
| `Documentation` | if doc comments |

## EnumCase
| Component | Notes |
|-----------|-------|
| `NodeKind::EnumCase` | always |
| `Name` | case name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Callable` | if has associated values |

## Protocol
| Component | Notes |
|-----------|-------|
| `NodeKind::Protocol` | always |
| `Name` | protocol name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Typed` | always (marker) |
| `Conformances` | if inherits from protocols |
| `TypeParams` | if type parameters |
| `WhereClause` | if where clause |
| `Attributes` | if attributes |
| `Documentation` | if doc comments |

## Extension
| Component | Notes |
|-----------|-------|
| `NodeKind::Extension` | always |
| `FileId` | source file |
| `ExtensionTarget` | the type being extended |
| `Conformances` | if adding conformances |
| `WhereClause` | if where clause |
| `Attributes` | if attributes |

No `Name` — extensions extend an existing type.

## Function
| Component | Notes |
|-----------|-------|
| `NodeKind::Function` | always |
| `Name` | function name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Callable` | always (params + receiver) |
| `TypeAnnotation` | return type (if declared) |
| `Valued` | body CodeBlock (if has body) |
| `Static` | if `static` modifier |
| `TypeParams` | if type parameters |
| `WhereClause` | if where clause |
| `Attributes` | if attributes |
| `Documentation` | if doc comments |

## Initializer
| Component | Notes |
|-----------|-------|
| `NodeKind::Initializer` | always |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Callable` | always (params, no receiver) |
| `Valued` | body (if has body) |
| `TypeParams` | if type parameters |
| `WhereClause` | if where clause |
| `Attributes` | if attributes |

## Deinit
| Component | Notes |
|-----------|-------|
| `NodeKind::Deinit` | always |
| `FileId` | source file |
| `Valued` | body (if has body) |

## Field
| Component | Notes |
|-----------|-------|
| `NodeKind::Field` | always |
| `Name` | field name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `TypeAnnotation` | field type |
| `Gettable` | always for stored; if getter for computed |
| `Settable` | if `var` (stored) or has setter (computed) |
| `Valued` | default value or getter body |
| `Static` | if `static` modifier |
| `Attributes` | if attributes |
| `Documentation` | if doc comments |

## Subscript
| Component | Notes |
|-----------|-------|
| `NodeKind::Subscript` | always |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Callable` | always (subscript params) |
| `TypeAnnotation` | return type |
| `Subscript` | always (marker) |
| `Gettable` | always |
| `Settable` | if has setter |
| `Static` | if `static` modifier |
| `TypeParams` | if type parameters |
| `WhereClause` | if where clause |
| `Attributes` | if attributes |

## TypeAlias
| Component | Notes |
|-----------|-------|
| `NodeKind::TypeAlias` | always |
| `Name` | alias name |
| `FileId` | source file |
| `Vis` | if visibility declared |
| `Typed` | always (marker) |
| `TypeAnnotation` | target type |
| `TypeParams` | if type parameters |
| `Attributes` | if attributes |

## Import
| Component | Notes |
|-----------|-------|
| `NodeKind::Import` | always |
| `FileId` | source file |
| `ModulePath` | module path segments |
| `ImportAlias` | if `as` alias |
| `ImportItems` | if selective import |

## TypeParameter
| Component | Notes |
|-----------|-------|
| `NodeKind::TypeParameter` | always |
| `Name` | parameter name |
| `FileId` | source file |
| `TypeAnnotation` | default type (if declared) |
