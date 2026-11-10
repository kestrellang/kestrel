// test: execution
// stdlib: true
// expect-stdout: true\ntrue\n�a\nfalse\n2\n

module Test

import std.files.(Path, Directory, File)
import std.numeric.(UInt8)

// A Path is the file system's bytes, not a String, so a name that isn't
// UTF-8 (0xFF here) round-trips through create, list and remove. `string`
// decodes lossily, and a NUL byte fails with an error instead of cutting
// the path short.
@main
func main() {
    let tmp = Directory.createTemporary(prefix: "kpath").unwrap();
    var raw = tmp.bytes.toOwned();
    raw.append(47);
    raw.append(255);
    raw.append(97);
    let odd = Path(bytes: raw);
    let _ = File.writeBytes(odd, Array[UInt8]().asSlice());
    println("\(odd.exists())");
    for e in Directory.entries(tmp).unwrap() {
        let entry = e.unwrap();
        println("\(entry.path == odd)");
        println(entry.name);
    }
    let _ = File.remove(odd);
    let _ = Directory.remove(tmp);
    let withNul: Array[UInt8] = [97, 0, 98];
    println("\(Path(bytes: withNul).exists())");
    println("\(Path(bytes: withNul).bytes.count - 1)");
}
