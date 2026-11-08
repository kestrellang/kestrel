// test: execution
// stdlib: true

// `lookup` resolves `localhost` and address literals without a network,
// and reports an unresolvable name as `HostNotFound` (not an errno).

module Test

import std.streams.(IoErrorKind)

@main
func main() -> lang.i64 {
    match lookup("localhost", 80) {
        .Ok(list) => {
            if list.count == 0 { return 1 }
            for address in list {
                if address.port != 80 or not address.ip.isLoopback { return 2 }
            }
        },
        .Err(_) => return 3
    }
    match lookup("127.0.0.1", 443) {
        .Ok(list) => if list.count != 1 or "\(list(0))" != "127.0.0.1:443" { return 4 },
        .Err(_) => return 5
    }
    match lookup("no-such-host.invalid", 80) {
        .Ok(_) => return 6,
        .Err(e) => if e.kind != IoErrorKind.HostNotFound { return 7 }
    }
    0
}
