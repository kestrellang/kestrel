// test: execution
// stdlib: true

// IPv4, IPv6 and socket addresses parse strictly and print in their
// canonical form (RFC 5952 for IPv6).

module Test

func roundTrip(text: String) -> String {
    match IpAddress.parse(text) { .Some(ip) => "\(ip)", .None => "none" }
}

@main
func main() -> lang.i64 {
    if roundTrip("127.0.0.1") != "127.0.0.1" { return 1 }
    if roundTrip("01.2.3.4") != "none" { return 2 }
    if roundTrip("1.2.3") != "none" { return 3 }
    if roundTrip("256.0.0.1") != "none" { return 4 }
    if roundTrip("::1") != "::1" { return 5 }
    if roundTrip("::") != "::" { return 6 }
    if roundTrip("2001:DB8:0:0:1:0:0:1") != "2001:db8::1:0:0:1" { return 7 }
    if roundTrip("::ffff:1.2.3.4") != "::ffff:102:304" { return 8 }
    if roundTrip("fe80::1%3") != "fe80::1%3" { return 9 }
    if roundTrip("1::2::3") != "none" { return 10 }
    if roundTrip("1:2:3:4:5:6:7:8:9") != "none" { return 11 }

    match SocketAddress.parse("[::1]:8080") {
        .Some(a) => if "\(a)" != "[::1]:8080" or a.port != 8080 or not a.ip.isLoopback { return 12 },
        .None => return 13
    }
    match SocketAddress.parse("10.0.0.1:80") {
        .Some(a) => if a.ip != IpAddress.V4(Ipv4Address(10, 0, 0, 1)) { return 14 },
        .None => return 15
    }
    if SocketAddress.parse("10.0.0.1:65536").isSome { return 16 }
    if SocketAddress.parse("10.0.0.1").isSome { return 17 }

    if not IpAddress.parse("127.5.0.1").unwrap().isLoopback { return 18 }
    if not IpAddress.V6(Ipv6Address.any).isUnspecified { return 19 }
    let (a, b, c, d) = Ipv4Address.localhost.octets;
    if a != 127 or b != 0 or c != 0 or d != 1 { return 20 }
    0
}
