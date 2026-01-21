# Modules and Imports

Kestrel uses a file-based module system where each file declares its module path and explicitly imports symbols from other modules. This system provides clear namespace management and explicit dependency tracking.

## Module Declarations

Every Kestrel source file must declare its module path using the `module` keyword.

### Syntax

```kestrel
module ModuleName
module Parent.Child
module Org.Project.Feature
```

### Single-Segment Modules

```kestrel
// main.ks
module Main

struct App {
    // ...
}
```

### Nested Module Paths

Module paths use dot notation to represent hierarchical namespaces:

```kestrel
// math_geometry.ks
module Math.Geometry

public struct Point {
    public var x: Float64;
    public var y: Float64;
}

public struct Circle {
    public var center: Point;
    public var radius: Float64;
}
```

The nested path `Math.Geometry` indicates that this module is conceptually part of a `Math` namespace, in a `Geometry` submodule.

### File-to-Module Mapping

**Important**: The module path is declared in the file itself, not determined by the file name or directory structure. The compiler uses the module declaration to organize the code namespace.

```kestrel
// File: lang/std/collections/array.ks
module std.collections  // Module path declared in file

public struct Array[T] {
    // ...
}
```

```kestrel
// File: lang/expressks/http.ks
module expressks.http   // Module path declared in file

public enum HttpMethod {
    case Get
    case Post
    // ...
}
```

## Import Statements

Import statements make symbols from other modules available in the current file.

### Import Entire Module

Import all public symbols from a module:

```kestrel
import Library
import Math.Geometry
```

After importing, you can use any public symbol from that module directly by name:

```kestrel
module Consumer

import Math.Geometry

func distance(p1: Point, p2: Point) -> Float64 {
    // Point is available from Math.Geometry
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    sqrt(dx * dx + dy * dy)
}
```

### Import Specific Items

Import only specific symbols from a module using selective import syntax:

```kestrel
import ModuleName.(Item1, Item2, Item3)
```

Examples:

```kestrel
import std.collections.(Array, Dictionary)
import Math.Geometry.(Point, Circle)
```

This imports only the specified items, making them available by name.

### Import with Module Alias

Import a module under a different name to avoid conflicts or provide shorter names:

```kestrel
import ModuleName as Alias
```

Examples:

```kestrel
module Consumer

import Math.Geometry as Geo

func makeCircle() -> Geo.Circle {
    Geo.Circle(
        center: Geo.Point(x: 0.0, y: 0.0),
        radius: 1.0
    )
}
```

**Note**: When using module aliases, you must qualify symbol names with the alias (e.g., `Geo.Circle`).

### Import with Item Alias

Import specific items with different names:

```kestrel
import ModuleName.(OriginalName as NewName)
```

Examples:

```kestrel
import ModuleA.(Widget as WidgetA)
import ModuleB.(Widget as WidgetB)

func compare() {
    let a = WidgetA();
    let b = WidgetB();
    // Use both without conflict
}
```

You can mix aliased and non-aliased imports:

```kestrel
import Library.(Foo, Bar as B, Baz)
```

### Public Imports (Re-exports)

Modules can re-export symbols from other modules using `public import`:

```kestrel
module expressks

// Re-export specific items
public import expressks.http.(
    HttpMethod,
    HttpStatus,
    Request,
    Response
)

// Re-export entire module's public symbols
public import expressks.router.Router

// Re-export with selective aliasing
public import expressks.server.(Server, ServerConfig)
```

Public imports make imported symbols available to consumers of the current module:

```kestrel
module MyApp

import expressks  // Gets HttpMethod, Request, Response, Router, Server, etc.

func handler(req: Request) -> Response {
    // Request and Response available via expressks
    Response.ok("Hello")
}
```

This is useful for:
- Creating a unified API from multiple submodules
- Library facades that consolidate exports
- Avoiding deep import paths for consumers

## Visibility and Access Control

### Visibility Levels

Kestrel has three visibility levels:

- **`public`**: Accessible from any module
- **`internal`**: Accessible only within the same module (default)
- **`private`**: Accessible only within the declaring type

### Public Symbols

Only `public` symbols can be imported from other modules:

```kestrel
// library.ks
module Library

public struct PublicClass {    // Can be imported
    public var x: Int;         // Accessible on imported instances
    internal var y: Int;       // Not accessible from other modules
    private var z: Int;        // Not accessible outside this type
}

internal struct InternalClass {  // Cannot be imported from other modules
    // ...
}

private struct PrivateClass {    // Cannot be imported (file-local)
    // ...
}
```

### Module Boundaries

Each module creates a visibility boundary:

```kestrel
// math_core.ks
module Math.Core

internal struct Helper {  // Only visible within Math.Core module
    // ...
}

public struct Calculator {
    public func add(a: Int, b: Int) -> Int {
        a + b
    }
}
```

```kestrel
// consumer.ks
module Consumer

import Math.Core.(Calculator)  // OK: Calculator is public
import Math.Core.(Helper)      // ERROR: Helper is internal
```

## Name Resolution

### Import Priority

When resolving names, Kestrel searches in this order:

1. Local declarations in the current scope (variables, parameters)
2. Imported symbols (from import statements)
3. Declarations in the current module
4. Built-in types

### Shadowing Rules

- Local declarations shadow imports
- Imports shadow module-level declarations
- Later imports do not shadow earlier imports (causes conflict error)

```kestrel
module Example

import Library.(Widget)

struct Widget {  // ERROR: 'Widget' is already imported
    // ...
}
```

### Qualified Access

You can always use fully qualified names to avoid ambiguity:

```kestrel
module Consumer

import ModuleA
import ModuleB

func test() {
    // If both modules export Widget, use qualified names:
    let a = ModuleA.Widget();
    let b = ModuleB.Widget();
}
```

However, this requires using module aliases since bare imports don't create namespace prefixes:

```kestrel
module Consumer

import ModuleA as A
import ModuleB as B

func test() {
    let a = A.Widget();  // Clear which module
    let b = B.Widget();  // Clear which module
}
```

## Examples

### Basic Module Organization

```kestrel
// geometry.ks
module Geometry

public struct Point {
    public var x: Float64;
    public var y: Float64;

    public init(x: Float64, y: Float64) {
        self.x = x;
        self.y = y;
    }
}

public struct Line {
    public var start: Point;
    public var end: Point;
}
```

```kestrel
// main.ks
module Main

import Geometry.(Point, Line)

func main() {
    let p1 = Point(x: 0.0, y: 0.0);
    let p2 = Point(x: 10.0, y: 10.0);
    let line = Line(start: p1, end: p2);
}
```

### Nested Module Hierarchy

```kestrel
// std_collections_array.ks
module std.collections

public struct Array[T] {
    // ...
}
```

```kestrel
// std_collections_dictionary.ks
module std.collections

public struct Dictionary[K, V] {
    // ...
}
```

```kestrel
// app.ks
module App

import std.collections.(Array, Dictionary)

func demo() {
    let arr = Array[Int]();
    let dict = Dictionary[String, Int]();
}
```

### Module Facade Pattern

Create a unified API from multiple submodules:

```kestrel
// lib.ks - Main library entry point
module mylib

// Re-export core types
public import mylib.core.(Context, Config)

// Re-export utilities
public import mylib.utils.(parse, format)

// Re-export client
public import mylib.client.Client

// Library-level convenience functions
public func createClient(config: Config) -> Client {
    Client(config: config)
}
```

```kestrel
// consumer.ks
module Consumer

import mylib  // Gets everything from the facade

func main() {
    let config = Config();
    let client = createClient(config: config);
}
```

### Avoiding Name Conflicts

```kestrel
module Graphics

import Math.Geometry.(Point as GeoPoint)
import Graphics.Rendering.(Point as RenderPoint)

func convert(geo: GeoPoint) -> RenderPoint {
    RenderPoint(
        x: geo.x,
        y: geo.y,
        color: 0xFF000000
    )
}
```

### Selective Imports

```kestrel
module DataProcessor

// Only import what we need
import std.collections.(Array)
import std.text.(String)
import std.result.(Result, Optional)
import std.io.(File, ReadError, WriteError)

func processFile(path: String) -> Result[Array[String], ReadError] {
    // Implementation uses only imported types
}
```

## Common Errors

### E0601: Module Not Found

```kestrel
module Test

import NonExistent  // ERROR
```

```
error[E0601]: module 'NonExistent' not found
  --> test.ks:3:8
   |
 3 | import NonExistent
   |        ^^^^^^^^^^^ no module named 'NonExistent'
   |
   = note: the module 'NonExistent' does not exist or is not visible from this scope
```

### E0602: Module Path Segment Not Found

```kestrel
module Test

import Library.Nonexistent  // ERROR: if Library.Nonexistent doesn't exist
```

```
error[E0602]: module 'Library.Nonexistent' not found
  --> test.ks:3:8
   |
 3 | import Library.Nonexistent
   |                ^^^^^^^^^^^ no module named 'Nonexistent'
   |        ------------------- in this import
   |
   = note: the module 'Library.Nonexistent' does not exist or is not visible from this scope
```

### E0603: Symbol Not Found in Module

```kestrel
module Test

import Library.(NonExistent)  // ERROR: if Library has no such symbol
```

```
error[E0603]: symbol 'NonExistent' not found in module 'Library'
  --> test.ks:3:17
   |
 3 | import Library.(NonExistent)
   |                 ^^^^^^^^^^^ 'NonExistent' does not exist
   |        ------- in module 'Library'
```

### E0604: Symbol Not Accessible

```kestrel
// library.ks
module Library

private struct PrivateClass {}
```

```kestrel
// consumer.ks
module Consumer

import Library.(PrivateClass)  // ERROR
```

```
error[E0604]: 'PrivateClass' is not accessible
  --> consumer.ks:3:17
   |
 3 | import Library.(PrivateClass)
   |                 ^^^^^^^^^^^^ 'PrivateClass' is private
   |
   = note: only public symbols can be imported from other modules
```

### E0605: Import Conflict

```kestrel
module Test

import LibraryA.(Widget)
import LibraryB.(Widget)  // ERROR: Widget already imported
```

```
error[E0605]: 'Widget' is already imported
  --> test.ks:4:17
   |
 3 | import LibraryA.(Widget)
   |                  ------ 'Widget' first imported here
 4 | import LibraryB.(Widget)
   |                  ^^^^^^ cannot import 'Widget'
   |
   = help: use an alias: import LibraryB.(Widget as WidgetB)
```

### E0606: Import Conflicts with Local Declaration

```kestrel
module Test

import Library.(Widget)

struct Widget {}  // ERROR: Widget already imported
```

```
error[E0606]: 'Widget' is already declared
  --> test.ks:5:8
   |
 3 | import Library.(Widget)
   |                 ------ 'Widget' imported here
 4 |
 5 | struct Widget {}
   |        ^^^^^^ 'Widget' is already declared
   |
   = help: rename the local declaration or use an import alias
```

### E0607: Duplicate Import

```kestrel
module Test

import Library.(Foo)
import Library.(Foo)  // ERROR
```

```
error[E0607]: 'Foo' is already imported
  --> test.ks:4:17
   |
 3 | import Library.(Foo)
   |                 --- 'Foo' first imported here
 4 | import Library.(Foo)
   |                 ^^^ 'Foo' is already imported
```

## Grammar

```ebnf
(* Module Declaration *)
ModuleDeclaration = "module" ModulePath ;

(* Module Path *)
ModulePath = Identifier { "." Identifier } ;

(* Import Declaration *)
ImportDeclaration = [ "public" ] "import" ModulePath [ ImportSuffix ] ;

ImportSuffix = ModuleAlias | ImportItems ;

ModuleAlias = "as" Identifier ;

ImportItems = "." "(" ImportItemList ")" ;

ImportItemList = ImportItem { "," ImportItem } [ "," ] ;

ImportItem = Identifier [ "as" Identifier ] ;

(* Visibility Modifiers *)
Visibility = "public" | "internal" | "private" ;

(* Top-level declarations can have visibility *)
VisibleDeclaration = [ Visibility ] Declaration ;
```

## Design Rationale

### Explicit Module Paths in Files

Module paths are declared in source files rather than inferred from directory structure. This provides:

- **Flexibility**: Files can be organized on disk differently than logical module hierarchy
- **Clarity**: Module path is explicit and visible when reading code
- **Tooling simplicity**: No need to maintain strict directory/module correspondence

### Selective Imports

Unlike languages with namespace-based access (e.g., `Math.Geometry.Point`), Kestrel requires explicit imports. This:

- Makes dependencies explicit and visible at the top of each file
- Reduces verbosity in code (use `Point` instead of `Math.Geometry.Point`)
- Enables better IDE support and error messages
- Encourages thoughtful module design

### Public Re-exports

The `public import` feature allows modules to:

- Create stable public APIs that hide internal organization
- Consolidate related functionality from submodules
- Evolve internal structure without breaking consumers
- Build layered libraries with clear entry points

### File-Based Modules

Each file is exactly one module (no nested module declarations). This ensures:

- Simple mental model: one file = one module path
- Clear ownership and organization
- Straightforward build systems
- Easy to locate code by module path

## Related

- [Syntax Guide](syntax.md) - Overview of all Kestrel syntax
- [Language Semantics](semantics.md) - Formal semantic rules
- [Visibility](semantics.md#visibility) - Detailed visibility rules
