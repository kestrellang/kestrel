// test: diagnostics
// stdlib: false

module Main

struct Id {
    let value: lang.i64

    init(cond: lang.i1) {
        if cond {
            self.value = 1;
        } else {
            self.value = 2;
        }
    }
}
