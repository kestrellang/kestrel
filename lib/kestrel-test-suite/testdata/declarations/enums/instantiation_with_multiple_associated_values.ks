// test: diagnostics
// stdlib: false
module Test
enum Event {
    case Click(x: lang.i64, y: lang.i64)
    case Scroll(delta: lang.f64)
}

func createEvent() -> Event {
    Event.Click(x: 100, y: 200)
}
