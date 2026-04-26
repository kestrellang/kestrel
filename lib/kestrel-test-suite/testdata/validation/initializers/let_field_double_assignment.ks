// test: diagnostics
// stdlib: false

module Main

struct Id {
    let value: lang.i64

    init() {
        self.value = 1;
        self.value = 2; // ERROR
    }
}
