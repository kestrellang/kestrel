// test: execution
// stdlib: true
// expect-exit: 0

module Test

// `Res[Int64, String].Err("e")`: `T` appears in no payload of this case, so only
// the written owner args can supply it.

enum Res[T, E] { case Ok(T) case Err(E) }

func describe(r: Res[Int64, String]) -> Int64 {
    match r { .Ok(v) => v, .Err(_) => 100 }
}

@main
func main() -> Int64 {
    let r = Res[Int64, String].Err("e");
    if describe(r) != 100 { return 1 }
    0
}
