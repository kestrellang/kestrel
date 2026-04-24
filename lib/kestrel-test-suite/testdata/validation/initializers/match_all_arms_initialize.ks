// test: diagnostics
// stdlib: false

module Main

enum Result[T, E] {
    case Ok(T)
    case Err(E)
}

struct Container[T] {
    var value: T

    init(result: Result[T, lang.i64]) {
        match result {
            .Ok(v) => {
                self.value = v;
            },
            .Err(_) => {
                self.value = lang.panic("failed");
            }
        }
    }
}
