# Pattern matching

Patterns appear in `match` arms, `if let`, `guard let`, `while let`, `let`
destructuring, `for` loops and parameter lists. A `match` must cover every
case (E305); `_` covers the rest.

## Pattern forms

| Pattern | Matches | Example |
|---|---|---|
| `_` | anything, binds nothing | `_ => 0` |
| `name` | anything, binds a copy | `x => x + 1` |
| `&name` / `&mutating name` | binds a reference into the value (writes land in place) | `if let .Filled(&mutating v) = s { v = 2; }` |
| literal | equal values | `0 =>`, `"yes" =>` |
| range | `a..<b`, `a..=b` | `1..<10 =>` |
| enum case | a case, with payload patterns | `.Circle(radius: r)`, `.Err(.Empty)` |
| Optional | `some p`, `null` | `some .Ok(v)`, `null =>` |
| tuple | element-wise, `..` skips | `(first, .., last)` |
| struct | named fields, `..` skips the rest | `Point { x: 0, y }`, `Point { .. }` |
| array / slice | by position; `..` skips, `..name` binds the rest as a slice | `[]`, `[only]`, `[first, ..rest]`, `[first, .., last]` |
| or | any alternative | `.Red or .Orange =>` |
| `@` | binds the whole value and matches a subpattern | `all @ .Filled(_)` |
| type test | an existential holding that type | `c is Circle => c.r`, `is Rect { w, .. } => w` |

Guards go after the pattern: `Point { x, y } if x == y => …`.

## Worked example

```kestrel
module Patterns

enum Color { case Red case Orange case Blue }
enum Slot { case Empty case Filled(Int64) }
struct Point { var x: Int64; var y: Int64 }

func warmth(c: Color) -> String {
    match c {
        .Red or .Orange => "warm",
        .Blue => "cool"
    }
}

func locate(p: Point) -> String {
    match p {
        Point { x: 0, y } => "on y-axis at \(y)",
        Point { x, y: 0 } => "on x-axis at \(x)",
        Point { x, y } if x == y => "diagonal",
        Point { .. } => "elsewhere"
    }
}

func describe(xs: &Slice[Int64]) -> String {
    match xs {
        [] => "empty",
        [only] => "one: \(only)",
        [first, .., last] => "\(first)…\(last)"
    }
}

func head(xs: &Slice[Int64]) -> Int64 {
    match xs {
        [first, ..rest] => first + rest.count,
        [] => 0
    }
}

@main
func main() -> Int64 {
    println(warmth(.Orange));
    println(locate(Point(x: 0, y: 4)));
    println(describe([1, 2, 3]));
    println(head([5, 6, 7]));
    var s = Slot.Filled(1);
    if let .Filled(&mutating v) = s { v = 2; }
    match s { .Filled(n) => println(n), .Empty => () };
    let r: Result[Int64?, String] = .Ok(3);
    match r { .Ok(some v) => println(v), .Ok(null) => println("none"), .Err(e) => println(e) };
    let a: Int64? = 1; let b: Int64? = 2;
    if let some x = a, let some y = b, x < y { println("\(x)<\(y)"); }
    let whole = match s { all @ .Filled(_) => all, .Empty => Slot.Empty };
    let (p, q) = (1, 2);
    let t = (1, 2, 3, 4);
    match t { (first, .., last) => println(first + last) };
    let n = 7;
    let size = match n { 0 => "zero", 1..<10 => "small", _ => "big" };
    println(size);
    p + q - 3
}
```

## Rules worth knowing

- **Write Optionals with `some x` and `null`**, not `.Some(x)` / `.None`.
  `some` takes any nested pattern: `some .Str(s)`, `some (a, b)`.
- **`if let` chains** take patterns and bools, separated by commas:
  `if let some x = a, let some y = b, x < y { … }`.
- **`guard let` must exit** in its `else` (`return`, `break`, `continue`, or
  a call returning `!`). Its bindings stay in scope after the guard.
- **`is` matches are never exhaustive.** Add `_`. An `is` binding borrows,
  so `var c is T` is E153.
- **Bindings copy by default.** Matching a move-only payload by name moves
  it. Use `&name` to look without moving, or match a `consuming` scrutinee.
- **Arms are separated by `,`.** A `match` used as a statement ends with `;`.
- **`where` is a keyword.** It can't be a function or variable name.
