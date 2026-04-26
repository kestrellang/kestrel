// test: diagnostics
// stdlib: false

module Main

struct Counter {
    var value: lang.i64

    init(cond: lang.i1) {
        self.value = 0;
        while cond {
            self.value = lang.i64_add(self.value, 1);
        }
    }
}
