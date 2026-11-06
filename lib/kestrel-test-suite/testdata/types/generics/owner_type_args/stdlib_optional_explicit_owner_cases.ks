// test: execution
// stdlib: true
// expect-exit: 0

module Test

// The real stdlib `Optional`: explicit-owner cases, with and without payload.

@main
func main() -> Int64 {
    let n = Optional[Int64].None;
    let s = Optional[Int64].Some(4);
    if let .Some(_) = n { return 1 }
    if let .Some(v) = s { if v != 4 { return 2 } } else { return 3 }
    let r = Result[Int64, String].Err("bad");
    if let .Ok(_) = r { return 4 }
    0
}
