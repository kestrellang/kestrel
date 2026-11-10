// test: execution
// stdlib: true
// expect-stdout: not found: /no-such-dir/config.ini\nSome(/no-such-dir/config.ini)\nnot found: /no-such-dir\nnot found: /no-such-dir/x\nnot found: /no-such-dir/y\ntrue\n
//
// A failed `std.files` call records the path it was about, and the error
// prints it after the reason. Equality ignores the path: two errors of the
// same kind and errno are equal whichever file they name.

module Test

@main
func main() {
    if let .Err(e) = File.readText("/no-such-dir/config.ini") {
        println(e);
        println(e.path);
    }
    if let .Err(e) = Directory.entries("/no-such-dir") { println(e); }
    if let .Err(e) = Path("/no-such-dir/x").metadata() { println(e); }
    if let .Err(a) = File.remove("/no-such-dir/y"), let .Err(b) = File.remove("/no-such-dir/z") {
        println(a);
        println(a == b);
    }
}
