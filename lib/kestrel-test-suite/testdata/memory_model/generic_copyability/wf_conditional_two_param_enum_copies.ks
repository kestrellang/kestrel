// test: execution
// stdlib: true

module Test

enum MyResult[T, E]: not Copyable {
    case Ok(T)
    case Err(E)
}

extend MyResult[T, E]: Copyable where T: Copyable, E: Copyable { }

@main
func main() -> lang.i64 {
    let r: MyResult[lang.i64, lang.i64] = .Ok(5);
    let a = r;
    let b = r;
    return 0;
}
