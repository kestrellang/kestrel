// test: diagnostics
// stdlib: false

module Main

enum Weekday {
    case Monday
    case Tuesday
    case Wednesday
    case Thursday
    case Friday
    case Saturday
    case Sunday
}

func isMonday(day: Weekday) -> lang.i1 {
    match day {
        .Monday => true,
        _ => false
    }
}
