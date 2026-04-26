// test: diagnostics
// stdlib: false

module Test
struct Mixed {
    let id: lang.i64
    let name: lang.str
    var value: lang.i64
    var count: lang.i64

    init(id: lang.i64, name: lang.str, value: lang.i64, count: lang.i64) {
        self.id = id;
        self.name = name;
        self.value = value;
        self.count = count;
    }
}
