// test: execution
// stdlib: false

// A defaulted parameter may be SKIPPED anywhere — leading, middle, or trailing
// — not only at the end. Arguments still bind in declaration order. Each
// parameter is encoded (years*10000 + months*100 + days) so a mis-bound
// argument produces a wrong total. Regression for the positional arg-binding
// bug; the single source of truth is kestrel-ast-builder/src/arg_binding.rs.
module Main

func adding(years y: lang.i64 = 0, months m: lang.i64 = 0, days d: lang.i64 = 0) -> lang.i64 {
    lang.i64_add(lang.i64_add(lang.i64_mul(y, 10000), lang.i64_mul(m, 100)), d)
}

func main() -> lang.i64 {
    // skip the leading defaulted `years`
    if lang.i64_ne(adding(months: 1, days: 10), 110) { return 1; }
    // skip the middle defaulted `months`
    if lang.i64_ne(adding(years: 1, days: 10), 10010) { return 2; }
    // omit the trailing `days`
    if lang.i64_ne(adding(years: 2, months: 3), 20300) { return 3; }
    // all explicit
    if lang.i64_ne(adding(years: 1, months: 2, days: 3), 10203) { return 4; }
    // none — all defaults
    if lang.i64_ne(adding(), 0) { return 5; }
    0
}
