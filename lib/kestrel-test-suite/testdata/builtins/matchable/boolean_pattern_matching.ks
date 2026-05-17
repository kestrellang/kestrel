// test: diagnostics
// stdlib: false

module Test
func describe(flag: lang.i1) -> lang.str {
    match flag {
        true => "enabled",
        false => "disabled"
    }
}
