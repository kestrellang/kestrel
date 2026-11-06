// A UTF-8 byte-order mark at the start of a file is whitespace (it was
// rejected as an unexpected character). No headers: the harness does not
// read them after a BOM, so this runs as a diagnostics test expecting none.

module Test

func f() -> Int64 { 0 }
