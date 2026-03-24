// test: diagnostics
// stdlib: false

module Main

func first[T](items: T, fallback: T = items) -> T { // ERROR: cannot reference
    fallback
}
