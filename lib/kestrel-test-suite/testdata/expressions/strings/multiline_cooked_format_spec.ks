// test: execution
// stdlib: true
// expect-exit: 0

module Test

func main() -> lang.i64 {
    let n = 42;
    let code = 0xCAFE;
    let s = """
        n     = \(n:>4)
        hex   = 0x\(code:04x)
        """;
    if s != "n     =   42\nhex   = 0xcafe" { return 1 }
    0
}
