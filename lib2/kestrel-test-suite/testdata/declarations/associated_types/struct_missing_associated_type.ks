// test: diagnostics
// stdlib: false

module Test
            protocol Iterator {
                type Item;
                func next() -> Item
            }
            struct BadIterator: Iterator {
                func next() -> lang.i64 { 0 }
            }
