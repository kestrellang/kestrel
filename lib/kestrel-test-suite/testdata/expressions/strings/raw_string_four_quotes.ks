// test: diagnostics
// stdlib: false

// Pound escalation: `##"..."##` lets the body contain `"#` literally,
// which a single-pound `#"..."#` form cannot.
module Main

func testEscalatedRawString() -> lang.str {
    ##"contains "# literal"##
}
