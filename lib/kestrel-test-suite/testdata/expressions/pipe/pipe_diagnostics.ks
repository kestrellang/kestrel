// test: diagnostics
// stdlib: true

module Test

func sub(a: Int64, b: Int64) -> Int64 { a - b }
func scale(by k: Int64) -> Int64 { k * 2 }
func double(x: Int64) -> Int64 { x * 2 }

func test() {
    // a call with no `_` is called with the piped value
    let a = 5 |> sub(1); // ERROR: did you mean `sub(_, 1)`?
    // not callable at all
    let b = 5 |> 3; // ERROR: not callable
    // a labeled first parameter needs the label in a slot
    let c = 5 |> scale; // ERROR: write `scale(by: _)`
    // `|>` binds loosest: this is `x |> (double ?? d)`
    let o: Int64? = .None;
    let d = o |> double ?? 0; // ERROR: write `(x |> f) ?? d`
}
