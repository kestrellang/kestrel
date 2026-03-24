// test: diagnostics
// stdlib: false

module Main

func identity[T](x: T) -> T { x }

func main() -> lang.i64 {
    identity[lang.i64](42)
}
