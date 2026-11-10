// test: execution
// stdlib: true
// expect-stdout: Ok(11)\nErr(4)\n

// `try` covers the whole postfix chain and unwraps only its final value:
// `try parse(s).mapErr(f)` maps the error first, then tries the result.
// A step in the middle is unwrapped only when parenthesized:
// `(try load()).port`.
module Test

struct Cfg { var port: Int64 }

func load(ok ok: Bool) -> Cfg throws String {
    if ok { .Ok(Cfg(port: 8)) } else { .Err("nope") }
}

func parse(s: String) -> Int64 throws String { .Ok(3) }

func run(ok ok: Bool) -> Int64 throws Int64 {
    let a = (try load(ok: ok).mapErr({ (e) in e.bytes.count })).port;
    let b = try parse("3").mapErr({ (e) in e.bytes.count });
    .Ok(a + b)
}

@main
func main() -> Int64 {
    println("\(run(ok: true))");
    println("\(run(ok: false))");
    0
}
