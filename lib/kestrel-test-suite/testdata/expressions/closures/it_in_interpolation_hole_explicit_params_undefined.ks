// test: diagnostics
// stdlib: true

module Test

// With EXPLICIT closure params, `it` is never injected — an `it` inside an
// interpolation hole must stay undefined, exactly like `it` in the plain
// body. Guards the `!has_explicit_params` gate on implicit-`it` injection
// now that interpolation holes are scanned during detection.

func applyStr(f: (Int64) -> String) -> String {
    f(7)
}

func run() -> String {
    applyStr({ (x) in "\(it)" }) // ERROR: undefined name 'it'
}
