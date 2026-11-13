# Deriving Conformances (`@derive`)

`@derive(P, ...)` on a struct or enum asks the compiler to write the
conformance for you. The derived code reads every stored field (or every
case and payload) and calls each one's own witness, so a derived `==`
compares field by field with each field's own `==`.

```kestrel
@derive(Equatable, Hashable, Formattable)
struct Version {
    var major: Int64
    var minor: Int64
}

let v = Version(major: 1, minor: 2);
println("\(v)");                              // Version(major: 1, minor: 2)
v == Version(major: 1, minor: 2)              // true
```

The generated conformance is an ordinary extension: it type-checks and
compiles like code you wrote by hand, and it is rebuilt whenever the type
changes.

## Derivable Protocols

| Protocol | What the derived code does |
|---|---|
| `Equatable` | Equal when every field is equal. Enums: same case and equal payloads. |
| `Hashable` | Hashes every field in order. Enums hash the case first, then its payload. |
| `Comparable` | Compares fields in declaration order; the first field that isn't `.Equal` decides. Enum cases compare by declaration order, then by payload. `<`, `>=` and the rest come from `Comparable`. |
| `Formattable` | Prints `Name(label: value, ...)`. An enum prints its case: `Circle(radius: 5)`, `Rect(1, 2)` for unlabeled payloads, `Empty` for no payload. A field-less struct prints `Name`. Width and alignment (`\(v:>20)`) apply to the whole value. |
| `Defaultable` | `init()` that sets each field to its initializer, or to its type's `init()` if it has none. An enum builds the one case marked `@default`, with default payloads. |
| `Cloneable` | `clone()` that copies every field: `Cloneable` fields are cloned, other `Copyable` fields are copied. Private fields and custom inits are fine. |

`Copyable` is never derived. Whether a type is `Copyable` follows from its
fields.

```kestrel
@derive(Equatable, Comparable, Formattable, Defaultable)
enum Level {
    case Low
    @default
    case Mid(Int64)
    case High(reason: String)
}

Level()                         // Mid(0)
Level.Low < Level.Mid(0)        // true: Low is declared first
"\(Level.High(reason: "hot"))"  // "High(reason: hot)"
```

A `String` field prints without quotes, because fields are formatted with
default options.

## Parent Protocols

A derived protocol's parents must hold too, but they are never derived for
you. `Hashable` and `Comparable` refine `Equatable`, so derive it alongside
them or write it yourself:

```kestrel
@derive(Hashable)              // E958: add 'Equatable' to '@derive'
struct Point { var x: Int64 }

@derive(Equatable, Hashable)   // OK
struct Point { var x: Int64 }
```

## Generic Types

On a generic type, a type parameter gets the bound `T: P` when a field's type
mentions it, and no bound otherwise:

```kestrel
@derive(Equatable)
struct Pair[T] {               // extend Pair[T]: Equatable where T: Equatable
    var a: T
    var b: T
}

@derive(Equatable)
struct Tagged[Tag] {           // Equatable for every Tag: no field uses it
    var id: Int64
}
```

`Cloneable` needs only `T: Copyable`, since it copies each field.

## On an Extension

`@derive` can also go on an extension of a type declared in the same file.
The extension's target and `where` clause are used as written, with no
inferred bounds:

```kestrel
struct Handle[T] { var raw: Int64 }

@derive(Equatable, Hashable)
extend Handle[T] { }           // Equatable for every T: only `raw` is compared
```

## Errors

| Code | When |
|---|---|
| E950 | The name in `@derive(...)` isn't a protocol. |
| E951 | The protocol has no deriver. The derivable ones are the six above. |
| E952 | A field or payload doesn't conform: `field 'f' has type 'T', which is not 'P'`. |
| E953 | The type already declares the protocol or implements one of its requirements, or `@derive` names it twice. |
| E954 | `@derive` on an extension of a type from another module or file; deriving for a type whose last field is unsized; deriving `Cloneable` for a type that opts out of `Copyable`. |
| E957 | An enum derives `Defaultable` without exactly one `@default` case. |
| E958 | A parent protocol doesn't hold. Add it to `@derive` or declare it. |

The full messages are in [`docs/error-codes.md`](../error-codes.md).
