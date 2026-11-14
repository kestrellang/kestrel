# References, slices and views

Full reference: the References page at https://kestrel-lang.com/docs.

## Taking inputs: pick the parameter type

| You need to… | Write | Callers can pass |
|---|---|---|
| read a sequence | `xs: &Slice[T]` | `Array`, a range of one (`xs(1..<3)`), `UnsafeSlice`, any `Indirection` to `Slice[T]`, an array literal |
| write elements in place (same length) | `xs: &mutating Slice[T]` | a `var` array or slice |
| grow/shrink an array | `mutating xs: Array[T]` | a `var` array |
| read text | `text: &StringSlice` | `String`, a string literal, a slice of either |
| own a value | `consuming x: T` | anything; the caller's binding is moved |
| read a small value | `x: T` (borrowing is the default) | anything |

Reach for `&Slice[T]` / `&StringSlice` before `Array[T]` / `String` for
read-only params. A `String` param still accepts a `&StringSlice` argument, but
copies it and warns (E512).

## Borrowing and ref types

```kestrel
var x = 10;
let r = &x;              // shared ref; only legal as a `let` initializer (E488 elsewhere)
let m = &mutating x;     // needs a var (E210)
m += 5;                  // writes x
```

- You never write `&` at a call site: arguments borrow by signature.
- `&T` may appear in return types, params, struct fields, tuple elements and
  generic args (`Optional[&T]`). Not in `let`/`var` annotations (E482) or
  function types.
- A ref can't outlive what it points into. The compiler tracks each ref's
  root; no lifetime annotations exist or are planned.

## Ref-returning members

```kestrel
module Refs

struct Counter {
    var n: Int64
    func peek() -> &Int64 { self.n }                   // name the place, no `&`
    mutating func slot() -> &mutating Int64 { self.n }
}

@main
func main() -> Int64 {
    var c = Counter(n: 1);
    c.slot() += 1;          // writes in place
    let copied = c.peek();  // binding a ref result copies it out ("decay")
    copied - 2
}
```

Properties and subscripts lend with accessors instead of a ref type:
`var value: T { ref { … } mutating ref { … } }`. On a failable member a
`ref` lends `Optional[&T]`.

## Slices and views

`Slice[T]` and `StringSlice` are **unsized**: you only ever hold them as
`&Slice[T]` / `&StringSlice`, borrowed from an owner.

```kestrel
let xs = [1, 2, 3, 4];
let tail = xs(1..);            // &Slice[Int64], borrows xs
let word = text.trimmed();     // &StringSlice, borrows text
let owned: String = text.trimmed();        // annotation converts to an owned copy
let parts: Array[Int64] = xs(0..<2);       // likewise (DecaysFrom)
let copy = word.toOwned();                 // explicit copy
```

- A slice ref never decays to a value on its own. It converts to `String` /
  `Array[T]` only where the target names that type: an annotation, a field, a
  return type, a `consuming` param.
- **A slice ref can't cross a branch merge** (E497): no returning one from
  different `if`/`match`/`guard` arms. Bind it before the branch, or return
  owned values from each arm.
- Collection literals need `Static` elements, so `[a.trimmed()]` is rejected
  unless the element type is named: `let xs: Array[String] = [a.trimmed()]`.

## Indirection: wrappers that act like their target

`Array` and `String` reach every `Slice` / `StringSlice` method through
`Indirection`. Member access that misses on the wrapper *peels* to
`value`, and a borrowed argument peels too. Your own wrapper gets the same:

```kestrel
module Views

struct Scores: MutableIndirection {
    var items: Array[Int64]
    type Target = Slice[Int64]
    var value: Slice[Int64] {
        ref { self.items.asSlice() }
        mutating ref { self.items.asMutableSlice() }
    }
}

func total(xs: &Slice[Int64]) -> Int64 {
    var sum = 0;
    for x in xs { sum += x; }
    sum
}

@main
func main() -> Int64 {
    let s = Scores(items: [5, 6]);
    println(s.count);          // Slice.count, through the peel
    total(s) - 11              // a borrowed argument peels too
}
```

- `Indirection` peels reads; `MutableIndirection` also peels writes and
  `mutating` calls.
- The wrapper's own members win a name clash.
- `consuming` arguments, bindings and returns never peel. Operators and
  protocol conformances don't either; forward them with an explicit `extend`.

## Generic code over unsized types

Every type parameter is implicitly `Sized`, `Static` and `Copyable`, so a
plain `T` can't be a slice. Opt in on a ref param:

```kestrel
func show[T](x: &T) -> String where T: Formattable, T: not Static, T: not Sized { "\(x)" }
func show2(x: &some Formattable and not Static and not Sized) -> String { "\(x)" }
```

`println` and string interpolation take their argument this way, so
`println(word)` prints a slice without copying.
