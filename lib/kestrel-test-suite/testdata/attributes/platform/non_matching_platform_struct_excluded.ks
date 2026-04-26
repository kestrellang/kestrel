// test: diagnostics
// stdlib: false

module Test

@platform(.linux)
struct ExcludedStruct {
    var x: lang.i64
}

func main() {
    let s = ExcludedStruct(x: 1); // ERROR: ExcludedStruct
}
