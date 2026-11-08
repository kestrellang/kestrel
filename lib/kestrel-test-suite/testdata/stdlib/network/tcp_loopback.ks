// test: execution
// stdlib: true

// A TCP listener on a picked loopback port, a client connecting by address
// and by name, shutdown, `tryClone`, a refused connection, and a write to a
// closed peer reported as an error instead of killing the process
// (SIGPIPE).

module Test

import std.streams.(IoErrorKind)

@main
func main() -> lang.i64 {
    let listener = match TcpListener.bind(to: SocketAddress.parse("127.0.0.1:0").unwrap()) {
        .Ok(l) => l,
        .Err(_) => return 1
    };
    let address = listener.localAddress().unwrap();
    if address.port == 0 { return 2 }

    var client = match TcpStream.connect(to: address, timeoutMilliseconds: 2000) { .Ok(c) => c, .Err(_) => return 3 };
    var (server, peer) = match listener.accept() { .Ok(p) => p, .Err(_) => return 4 };
    if peer.ip != IpAddress.V4(Ipv4Address.localhost) { return 5 }
    if client.peerAddress().unwrap() != address { return 6 }

    if client.setNoDelay(true).isErr { return 7 }
    if client.write(text: "hello").isErr { return 8 }
    if client.shutdown(.Write).isErr { return 9 }
    match server.readText() { .Ok(t) => if t != "hello" { return 10 }, .Err(_) => return 11 }

    // A clone has its own descriptor; closing it leaves the original open.
    var other = server.tryClone().unwrap();
    if other.write(text: "bye").isErr { return 12 }
    if other.close().isErr { return 13 }
    if server.write(text: "!").isErr { return 14 }
    if server.shutdown(.Both).isErr { return 15 }
    match client.readText() { .Ok(t) => if t != "bye!" { return 16 }, .Err(_) => return 17 }

    // Connecting by name tries each address of `localhost`.
    var named = match TcpStream.connect("localhost", address.port) { .Ok(c) => c, .Err(_) => return 18 };
    let (dropped, _) = listener.accept().unwrap();
    let _ = dropped.close();
    var broken = false;
    var tries = 0;
    while tries < 100 and not broken {
        match named.write(text: "x") {
            .Ok(_) => (),
            .Err(e) => {
                if e.kind != IoErrorKind.BrokenPipe and e.kind != IoErrorKind.ConnectionReset { return 19 }
                broken = true
            }
        }
        tries = tries + 1
    }
    if not broken { return 20 }

    match TcpStream.connect(to: closedAddress()) {
        .Ok(_) => return 21,
        .Err(e) => if e.kind != IoErrorKind.ConnectionRefused { return 22 }
    }
    0
}

/// A loopback address nothing listens on: the listener closes on return.
func closedAddress() -> SocketAddress {
    let listener = TcpListener.bind(to: SocketAddress.parse("127.0.0.1:0").unwrap()).unwrap();
    listener.localAddress().unwrap()
}
