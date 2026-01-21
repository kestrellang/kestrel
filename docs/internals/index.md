# Compiler Internals

Documentation on the Kestrel compiler architecture and language semantics.

## Compiler Architecture

- **[Parser Architecture](parser-architecture.md)** - Event-driven parsing with Chumsky
- **[Execution Graph](execution-graph.md)** - Dependency-driven compilation
- **[Validation Passes](validation-passes.md)** - Semantic analysis pipeline
- **[Type Inference Design](type-inference-design.md)** - Bidirectional type inference
- **[Monomorphization Codegen](monomorphize-codegen.md)** - Generic specialization
- **[Language Intrinsics](lang-intrinsics.md)** - Built-in compiler primitives
- **[Error Span Patterns](error-span-patterns.md)** - Diagnostic source locations

## Language Semantics

Formal definitions of language constructs.

### Core Constructs
- [Modules](modules.md) - Module declarations and organization
- [Imports](imports.md) - Import statements and module access
- [Types](types.md) - Type system overview
- [Type Aliases](type-aliases.md) - Type alias declarations
- [Closures](closures.md) - Closure syntax and capture

### Declarations
- [Functions](functions.md) - Function declarations and overloading
- [Structs](structs.md) - Struct declarations
- [Protocols](protocols.md) - Protocol declarations
- [Fields](fields.md) - Field declarations

### Resolution & Visibility
- [Visibility](visibility.md) - Access control system
- [Name Resolution](name-resolution.md) - How names are resolved
- [Type Resolution](type-resolution.md) - How types are resolved

### Reference
- [Errors](errors.md) - Complete error catalog
