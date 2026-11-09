// test: execution
// stdlib: true
// expect-stdout: true\ntrue\ntrue\n

// `sleep(for:)` pauses at least the given time, and a `Stopwatch` (on the
// monotonic clock) measures it. `restart()` returns the lap and resets.
// Upper bounds are loose so a busy machine doesn't fail the test.

module Test

@main
func main() {
    var watch = Stopwatch.start();
    sleep(for: .milliseconds(50));
    let lap = watch.restart();
    println(lap.totalMilliseconds >= 50 and lap.totalMilliseconds < 5000);
    // Right after a restart, almost nothing has elapsed.
    println(watch.elapsed.totalMilliseconds < 50);
    // Zero and negative durations return at once.
    sleep(for: .zero);
    sleep(for: Duration.seconds(-1));
    println(watch.elapsed.totalMilliseconds < 1000);
}
