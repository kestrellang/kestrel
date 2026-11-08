// test: execution
// stdlib: true

// UDP datagrams between two loopback sockets, connected UDP, a Unix
// socket pair, and a Unix listener on a path.

module Test

@main
func main() -> lang.i64 {
    let any = SocketAddress.parse("127.0.0.1:0").unwrap();
    var a = match UdpSocket.bind(to: any) { .Ok(s) => s, .Err(_) => return 1 };
    var b = match UdpSocket.bind(to: any) { .Ok(s) => s, .Err(_) => return 2 };
    let bAddress = b.localAddress().unwrap();
    let ping = "ping".asByteSlice();
    match a.send(unsafe Slice.lend(start: ping.pointer, count: ping.count), to: bAddress) {
        .Ok(n) => if n != 4 { return 3 },
        .Err(_) => return 4
    }
    var buf = Array[UInt8](repeating: 0, count: 64);
    match b.receiveFrom(into: buf.asMutableSlice()) {
        .Ok((n, from)) => if n != 4 or from != a.localAddress().unwrap() { return 5 },
        .Err(_) => return 6
    }

    // Connected: `send(buf)` goes to the fixed peer.
    if a.connect(to: bAddress).isErr { return 7 }
    if a.send(unsafe Slice.lend(start: ping.pointer, count: 2)).isErr { return 8 }
    if b.setReadTimeout(milliseconds: .Some(2000)).isErr { return 9 }
    match b.receive(into: buf.asMutableSlice()) {
        .Ok(n) => if n != 2 { return 10 },
        .Err(_) => return 11
    }

    var (left, right) = match UnixStream.pair() { .Ok(p) => p, .Err(_) => return 12 };
    if left.write(text: "unix").isErr { return 13 }
    if left.shutdown(.Write).isErr { return 14 }
    match right.readText() { .Ok(t) => if t != "unix" { return 15 }, .Err(_) => return 16 }

    let dir = Directory.createTemporary(prefix: "kestrel-unix").unwrap();
    let path = dir / Path("socket");
    let listener = match UnixListener.bind(path) { .Ok(l) => l, .Err(_) => return 17 };
    var client = match UnixStream.connect(path) { .Ok(c) => c, .Err(_) => return 18 };
    var server = match listener.accept() { .Ok(s) => s, .Err(_) => return 19 };
    if client.write(line: "over unix").isErr { return 20 }
    if client.shutdown(.Write).isErr { return 21 }
    match server.readText() { .Ok(t) => if t != "over unix\n" { return 22 }, .Err(_) => return 23 }
    if Directory.removeAll(dir).isErr { return 24 }
    0
}
