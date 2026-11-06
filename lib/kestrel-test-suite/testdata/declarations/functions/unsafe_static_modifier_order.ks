// test: execution
// stdlib: true
// expect-exit: 0

// `static` and `unsafe` may come in either order on a member.
module Test

struct S {
    unsafe static func a() -> Int64 { 1 }
    static unsafe func b() -> Int64 { 2 }
}

@main
func main() -> Int64 {
    unsafe S.a() + unsafe S.b() - 3
}
