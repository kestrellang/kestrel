// test: diagnostics
// stdlib: false

module Test

struct Table {
    @unavailable("needs 'mutating get' (0.20)", replacement: "modify")
    func insertDefault() {}

    func modify() {}
}

@unavailable
struct Gone {}

func use(t: Table) {
    t.insertDefault(); // ERROR: 'insertDefault' is unavailable: needs 'mutating get' (0.20); use 'modify' instead
}

func sig(g: Gone) {} // ERROR: 'Gone' is unavailable
