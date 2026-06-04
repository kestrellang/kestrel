// test: execution
// stdlib: true

module Test

enum State {
    case Off
    case On
}

func get_value(s: State) -> std.numeric.Int64 {
    match s {
        .Off => 0,
        .On => 1
    }
}

func toggle(mutating s: State) {
    s = match s {
        .Off => State.On,
        .On => State.Off
    };
}

@main
func main() -> lang.i64 {
    var state = State.Off;
    if get_value(state) != 0 { return 1 }

    toggle(state);
    if get_value(state) != 1 { return 2 }

    toggle(state);
    if get_value(state) != 0 { return 3 }

    0
}
