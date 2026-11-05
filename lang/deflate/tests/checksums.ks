// Known-answer checks for CRC-32 and Adler-32.

module deflate_tests_checksums

import deflate.(Crc32, Adler32)

func checkValue(name: String, got: UInt32, want: UInt32) {
    if got == want { print("PASS \(name)\n"); } else { print("FAIL \(name)\n  got  \(got)\n  want \(want)\n"); }
}

@main
func main() {
    checkValue("crc32 empty", Crc32.checksum("".toBytes()), 0);
    checkValue("crc32 123456789", Crc32.checksum("123456789".toBytes()), 0xCBF43926);
    checkValue("adler32 empty", Adler32.checksum("".toBytes()), 1);
    checkValue("adler32 Wikipedia", Adler32.checksum("Wikipedia".toBytes()), 0x11E60398);

    // Incremental updates match one-shot.
    var crc = Crc32();
    crc.update("12345".toBytes());
    crc.update("6789".toBytes());
    checkValue("crc32 incremental", crc.value, 0xCBF43926);

    // Adler-32 over more than 5552 bytes exercises the deferred modulo.
    var big: [UInt8] = [];
    for _ in 0..<20000 { big.append(255); }
    checkValue("adler32 20000 x 0xFF", Adler32.checksum(big), 0x9F51D664);
}
