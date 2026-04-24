// test: diagnostics
// stdlib: false

module Test
            protocol Dictionary {
                type Key;
                type Value;
                func read(key: Key) -> Value
            }
            struct StringIntMap: Dictionary {
                type Key = lang.str;
                type Value = lang.i64;
                func read(key: lang.str) -> lang.i64 { 0 }
            }
