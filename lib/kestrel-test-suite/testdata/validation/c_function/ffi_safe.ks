// test: diagnostics
// stdlib: true

// A C function type is FFISafe when its params and return are; a `&T`
// param is a C pointer.

module Test

struct Good: FFISafe { var cb: @extern(.C) (Int64, &Int32) -> UInt8 }
struct Bad: FFISafe { var cb: @extern(.C) (String) -> () } // ERROR: fields of 'Bad' do not conform to FFISafe: cb

@extern(.C, mangleName: "take")
func take(f: @extern(.C) (String) -> ()) // ERROR: parameter 'f' type does not conform to FFISafe

@main
func main() {}
