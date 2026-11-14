# Types

Full reference: `docs/language/types.md`, `generics.md`, `protocols.md`,
`existentials.md`, `extensions.md`, `derive.md`, `string-interpolation.md`.

## Primitives and sugar

- Integers: `Int8` `Int16` `Int32` `Int64`, `UInt8` `UInt16` `UInt32` `UInt64`.
  Literals default to `Int64`; annotate for others (`let b: UInt8 = 255;`).
- Floats: `Float32`, `Float64` (literal default). There is no `Float16`.
- `Bool`, `Char` (`'a'`; a char literal also types as `UInt8`), `String`,
  unit `()`, never `!`.
- Sugar: `T?` = `Optional[T]`, `[T]` = `Array[T]`, `[K: V]` = `Dictionary[K, V]`,
  `T throws E` = `Result[T, E]`.
- Tuples: `(Int64, String)`, `p.0`, destructuring `let (a, b) = p;`, swap
  `(a, b) = (b, a);`.
- Function types: `(Int64) -> Bool`, with an optional kind prefix
  (see [closures.md](closures.md)).
- Aliases: `type ID = String`, `type Pair[T] = (T, T)`.
- Numeric protocols for generic math: `Numeric`, `SignedNumeric`, `Integer`,
  `SignedInteger`, `UnsignedInteger`, `FloatingPoint` (`std.numeric`).

## Strings

| Form | Multi-line | Escapes | Interpolation |
|---|---|---|---|
| `"..."` | no | yes | yes |
| `"""` newline … `"""` | yes, indent stripped to the closing `"""` column | yes | yes |
| `#"..."#`, `##"..."##` | no | no | no |
| `#"""` … `"""#` | yes | no | no |

Raw forms are fully raw (no `\#(…)`). Pick the smallest `#` count whose
closer doesn't appear in the body. A multi-line content line indented less
than the closing `"""` is E704.

Interpolation: `"\(x)"`, with an optional spec after `:`:
`>` `<` `^` align, `0` zero-pad, width, `.n` precision, `x` `X` `b` `o`
radix, `?` debug. Width or precision can come from a name in braces:
`"\(n:>{width})"`. A type interpolates if it conforms to `Formattable`.

```kestrel
"\(age:>5)"     // "    7"
"\(code:08x)"   // "000000ff"
"\(price:.2)"   // "1.50"
"\(value:?)"    // debug form
```

## Protocols

```kestrel
protocol Drawable {
    func draw()
    mutating func reset()
    static func make() -> Self
    var name: String { get }
    type Output                    // associated type
}

protocol Container[T] { func read() -> T }        // generic protocol
struct Circle: Drawable { … }                      // conform at declaration
extend Circle: Hashable { … }                      // or in an extension
extend Drawable { func redraw() { self.draw(); } } // default implementation
```

Operators come from protocols (`Addable`, `Equatable`'s `isEqual(to:)`,
`Comparable`'s `compare`, …): conform and the operator works. See
"Operator Protocols" in `docs/language/protocols.md`.

## Generics and implicit bounds

```kestrel
struct Box[T] { var value: T }
struct Sorted[T] where T: Comparable { var items: [T] }
func largest[T](xs: &Slice[T]) -> T? where T: Comparable { xs.iter().max() }
func sum[S](xs: S) -> Int64 where S: Iterable, S.Item = Int64 { … }
```

Every type parameter is implicitly:

| Bound | Means | Relax with |
|---|---|---|
| `Copyable` | can be bit-copied | `where T: not Copyable` (move-only values) |
| `Static` | holds no borrowed refs | `where T: not Static` (views, slices) |
| `Sized` | known size | `where T: not Sized`, only behind `&T` (see [references.md](references.md)) |

## Extensions

```kestrel
extend Point { func dist() -> Float64 { … } }
extend Box[T] where T: Equatable { func same(other: Box[T]) -> Bool { … } }
extend Box[Int64] { func doubled() -> Int64 { self.value * 2 } }
extend[T] Int64: ArrayIndex[T] { … }    // declare params the target doesn't bind
extend[T] SeqRange: SliceIndex[T] { … } // blanket conformance for every SeqRange
extend &T: Describe where T: Describe { … }   // conform a reference type
```

There's no implicit introduction of single-letter params: in
`extend Int64: Reads[V]`, `V` names a type. Declare it: `extend[V] Int64: Reads[V]`.

## Existentials (`any P`)

```kestrel
let shapes: Array[any Shape] = [Square(s: 2), Rect(w: 1, h: 3)];  // boxed automatically
let owned: any[OwnedBox] Shape = Square(s: 3);                    // explicit box
let both: any Shape and Printable = Square(s: 1);                 // composition
func sum(xs: any Iterator[Item = Int64]) -> Int64 { … }           // pinned assoc type

if let some r = s as? Rect { … }          // borrows: Optional[&Rect]
if s is Rect { … }
match s { r is Rect => r.w, _ => 0 }       // `is` matches are never exhaustive
match consuming o as? Rect { .Ok(r) => …, .Err(back) => … }   // needs any[OwnedBox]
```

- The box is part of the type. `any P` is shared (`RcBox`: copies share the
  payload, so a `mutating` call shows through every copy). `any[CowBox] P`
  forks on write (needs `P: Cloneable`). `any[OwnedBox] P` is single-owner,
  the only one that allows `consuming` requirements and `consuming as?`.
- Different boxes are different types (E147).
- Not callable through `any P`: static and init requirements, generic ones,
  ones using `Self` as a param or return (`==`), ones mentioning an unpinned
  associated type.
- An `is` binding borrows, so `var c is T` is E153.
- `some P` in a return type is an opaque type: one concrete type, hidden.

## Copy, clone, move

- Plain structs of `Copyable` fields are `Copyable`.
- A struct holding `String` / `Array` / `Dictionary` is `Cloneable`. Copies
  happen through `clone()`, which the compiler inserts. `@derive(Cloneable)`
  writes it.
- `not Copyable` types move. `deinit { … }` runs once, when the owner drops.
- Memory model in depth: `docs/memory-model/`.
