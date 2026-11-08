// test: diagnostics
// stdlib: false

module Test

@unavailable
func gone() {}

@unavailable
func alsoGone() {
    gone(); // ERROR: 'gone' is unavailable
}

@deprecated
func old() {}

// Unavailable code is never run, so using a deprecated API from it is quiet.
@unavailable
func quiet() {
    old();
}
