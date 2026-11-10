// test: diagnostics
// stdlib: true

// `try load().port` is `try (load().port)`: the member is looked up on the
// `Result` itself, which has no `port`. Unwrapping mid-chain takes
// parentheses: `(try load()).port`.
module Test

struct Cfg { var port: Int64 }

func load() -> Cfg throws String { .Ok(Cfg(port: 8)) }

func port() -> Int64 throws String {
    let p = try load().port;   // ERROR: no member 'port' on type 'Result[Cfg, String[SystemAllocator]]'
    .Ok(p)
}
