# Array Literals Design

## Overview

Array literals (`[1, 2, 3]`) are implemented through a two-layer protocol system:

1. **`_ExpressibleByArrayLiteral`** - Low-level compiler interface using raw `lang.ptr` and `lang.i64`
2. **`ExpressibleByArrayLiteral`** - User-facing protocol using `LiteralSlice[Element]`

## Type Hierarchy

```
_ExpressibleByArrayLiteral          <- Compiler calls this
    ↑
LiteralSlice[T]                     <- Wraps raw pointer + count
    ↑
ExpressibleByArrayLiteral           <- User types conform to this
    ↑
Array[T, A], Set[T], etc.           <- Collection types
```

## Protocols

```kestrel
@builtin(._ExpressibleByArrayLiteral)
protocol _ExpressibleByArrayLiteral {
    type Element
    init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64)
}

@builtin(.ExpressibleByArrayLiteral)
protocol ExpressibleByArrayLiteral: _ExpressibleByArrayLiteral {
    type Element
    init(arrayLiteral: LiteralSlice[Element])
}

// Default implementation bridges the two
extend ExpressibleByArrayLiteral {
    init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(
            pointer: _arrayLiteralPointer,
            count: _arrayLiteralCount
        ))
    }
}
```

## LiteralSlice

A read-only view into compiler-allocated literal data:

```kestrel
struct LiteralSlice[T]: Iterable {
    private var ptr: lang.ptr[T]
    private var len: lang.i64

    var count: Int { Int(self.len) }
    var isEmpty: Bool { self.len == 0 }
    func iter() -> LiteralSliceIterator[T]
}
```

- No public constructor (only created by compiler or via protocol conformance)
- Read-only (no mutable pointer access)
- Iterable for easy consumption

## Default Type

When an array literal has no type context (`let x = [1, 2, 3]`), the compiler uses `Array[T, GlobalAllocator]` where `T` is inferred from element types.

This is special-cased in the compiler (not a generic type alias).

## Compilation Pipeline

```
Source: let arr: Array[Int] = [1, 2, 3]

1. Parser: ExprKind::Array([1, 2, 3])

2. Type Inference:
   - Target type: Array[Int, GlobalAllocator]
   - Generate constraint: Array[Int, _] conforms to _ExpressibleByArrayLiteral
   - Resolve Element = Int

3. Codegen:
   - Allocate buffer for [1, 2, 3] (stack or static)
   - Get pointer to buffer: lang.ptr[Int]
   - Get count: 3 as lang.i64
   - Call Array._ExpressibleByArrayLiteral.init(ptr, count)
   - (Default impl wraps in LiteralSlice, calls init(arrayLiteral:))
```

## Memory Model

Literal data allocation:
- **Constant literals** (`[1, 2, 3]`): May be placed in static memory
- **Runtime literals** (`[x, y, z]`): Stack allocated

Lifetime: The buffer is valid for the duration of the `init` call. Conforming types must copy data if they need to retain it.
