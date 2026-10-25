// test: execution
// backends: cranelift,llvm
// expect-exit: 0

// String-literal patterns test an owned scrutinee through a fresh borrow per
// arm. They used to test a value computed once before the chain (stale after
// the first branch consumed it) and to leak owned projections.

module Test

enum Tag { case named(String) case none }

func s() -> String { "b" }

func classify(consuming v: String) -> Int64 {
    match v { "x" => 1, "y" => 2, _ => 3 }
}

@main
func main() -> lang.i64 {
    let a = match s() { "a" => 1, "b" => 2, _ => 3 };
    if a != 2 { return 1 }

    let local = "b";
    let b = match local { "a" => 1, "b" => 2, _ => 3 };
    if b != 2 { return 2 }

    let t = match ("ab", 1) { ("ab", _) => 1, _ => 2 };
    if t != 1 { return 3 }

    let o: Optional[String] = .Some("a");
    let c = match o { .Some("a") => 1, .Some(_) => 2, .None => 3 };
    if c != 1 { return 4 }

    if let .Some("a") = o { } else { return 5 }

    if classify("y") != 2 { return 6 }

    let tag = Tag.named("q");
    let d = match tag { .named("q") => 1, .named(_) => 2, .none => 3 };
    if d != 1 { return 7 }

    let arr: [String] = ["a"];
    let e = match arr { ["a"] => 1, _ => 0 };
    if e != 1 { return 8 }
    0
}
