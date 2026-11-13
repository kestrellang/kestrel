// test: execution
// stdlib: true
// expect-exit: 0

// `map`, `andThen`, `flatten`, `andValue` and `zip` on Optional and Result
// accept a result type that isn't Static, such as a ref or a struct holding
// one. They used to require `U: Static`, so mapping to a borrowed view
// failed with "V !: Static".

module Test

import std.numeric.Int64

struct Holder {
    var n: Int64
}

func pick[V](v: V, o: Int64?) -> V? where V: not Static {
    o.map { x in v }
}

func pickResult[V](v: V, r: Result[Int64, Int64]) -> Result[V, Int64] where V: not Static {
    r.andThen { x in .Ok(v) }
}

@main
func main() -> lang.i32 {
    let h = Holder(n: 7);
    let r = &h;
    guard let .Some(got) = pick(r, .Some(1)) else { return 10 }
    if got.n != 7 { return 11 }
    guard let .None = pick(r, .None) else { return 12 }
    guard let .Ok(res) = pickResult(r, .Ok(2)) else { return 13 }
    if res.n != 7 { return 14 }
    let both = Optional.Some(1).zip(with: pick(r, .Some(1)));
    guard let .Some((one, ref)) = both else { return 15 }
    if one + ref.n != 8 { return 16 }
    0
}
