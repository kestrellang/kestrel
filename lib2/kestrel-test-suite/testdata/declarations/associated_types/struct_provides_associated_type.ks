// test: diagnostics
// stdlib: false

module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct IntIterator: Iterator {
                type Item = lang.i64;
                func next() -> lang.i64 { 0 }
            }
