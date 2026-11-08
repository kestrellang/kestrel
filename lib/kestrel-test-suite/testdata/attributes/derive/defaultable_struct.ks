// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// `@derive(Defaultable)` on a struct: each field takes its declared
// initializer, else `Field()`. The derived `init()` is an extra
// initializer: the memberwise one stays.

module Main

import std.numeric.(Int64)

@derive(Defaultable)
struct Config {
    var retries: Int64 = 3
    var name: String
    var tags: Array[String]
    let limit: Int64 = 10
}

@main
func main() -> Int64 {
    let c = Config();
    if c.retries != 3 { return 1 }
    if c.name != "" { return 2 }
    if c.tags.count != 0 { return 3 }
    if c.limit != 10 { return 4 }
    let d = Config(retries: 5, name: "x", tags: ["a"], limit: 1);
    if d.retries != 5 { return 5 }
    if d.name != "x" { return 6 }
    0
}
