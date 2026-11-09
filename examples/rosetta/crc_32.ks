// Rosetta Code: CRC-32
// https://rosettacode.org/wiki/CRC-32
module Main

func crc32(data: Array[UInt8]) -> UInt32 {
    // Table-driven, reflected polynomial 0xEDB88320.
    var table = Array[UInt32](repeating: 0, count: 256);
    for i in 0..<256 {
        var c = UInt32(truncating: i);
        for _ in 0..<8 {
            c = if c & 1 == 1 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        table(i) = c;
    }
    var crc: UInt32 = 0xFFFFFFFF;
    for byte in data {
        let index = Int64(from: (crc ^ UInt32(from: byte)) & 0xFF);
        crc = table(index) ^ (crc >> 8);
    }
    crc ^ 0xFFFFFFFF
}

@main
func main() {
    let crc = crc32("The quick brown fox jumps over the lazy dog".toBytes());
    println("\(crc:08x)");
}
