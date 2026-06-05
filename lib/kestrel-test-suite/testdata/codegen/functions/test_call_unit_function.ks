// test: execution
// stdlib: true

module Test

func do_nothing() {
}

@main
func main() -> lang.i64 {
    do_nothing();
    0
}
