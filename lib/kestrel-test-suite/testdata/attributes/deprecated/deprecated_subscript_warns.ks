// test: diagnostics
// stdlib: false

module Test

struct Table {
    var first: lang.i64

    @deprecated("use 'at(_:)'")
    subscript(slot index: lang.i64) -> lang.i64 {
        get { self.first }
    }
}

func read(t: Table) -> lang.i64 {
    t(slot: 0) // WARN: 'Table.subscript' is deprecated: use 'at(_:)'
}
