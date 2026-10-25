// test: diagnostics
// stdlib: false

module Test

@deprecated("unsafe on empty input", since: "0.18", replacement: "parse")
func legacyParse() -> lang.i64 { 0 }

@deprecated
func old() -> lang.i64 { 1 }

func caller() -> lang.i64 {
    let a = legacyParse(); // WARN: 'legacyParse' is deprecated since 0.18: unsafe on empty input; use 'parse' instead
    let b = old(); // WARN: 'old' is deprecated
    a
}
