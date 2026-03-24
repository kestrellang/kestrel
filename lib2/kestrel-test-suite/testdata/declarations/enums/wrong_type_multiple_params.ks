// test: diagnostics
// stdlib: false

module Test

enum Event {
    case Click(x: lang.i64, y: lang.i64)
}

func test() -> Event {
    Event.Click(x: 10, y: "twenty") // ERROR: does not conform to protocol
}
