// test: execution
// stdlib: true

// Three file-system footguns, each once confirmed by a probe:
// - `Directory.removeAll(link)` followed a symlink argument and emptied the
//   directory it pointed to. Now the link is removed and the target kept.
// - `copy(a, to: a)` (or onto a symlink or hard link to `a`) truncated the
//   destination before reading it, leaving `a` empty. Now it's InvalidInput.
// - A path holding a NUL byte named the shorter path before it, since C
//   stops there. Now it's InvalidInput.

module Test

import std.streams.(IoErrorKind)

@main
func main() -> lang.i64 {
    let dir = match Directory.createTemporary(prefix: "kestrel-footguns") { .Ok(d) => d, .Err(_) => return 1 };
    let precious = dir / Path("precious");
    if Directory.create(precious).isErr { return 2 }
    let data = precious / Path("data.txt");
    if writeText(data, "0123456789").isErr { return 3 }

    // removeAll on a symlink to a directory.
    let link = dir / Path("link");
    if createSymbolicLink(at: link, to: precious).isErr { return 4 }
    if Directory.removeAll(link).isErr { return 5 }
    if not data.exists() { return 6 }
    match symbolicLinkMetadata(link) { .Ok(_) => return 7, .Err(_) => {} }

    // copy onto the same file, directly and through a symlink.
    match copy(data, to: data) {
        .Ok(_) => return 8,
        .Err(e) => if e.kind != IoErrorKind.InvalidInput { return 9 }
    }
    let alias = dir / Path("alias.txt");
    if createSymbolicLink(at: alias, to: data).isErr { return 10 }
    match copy(data, to: alias) {
        .Ok(_) => return 11,
        .Err(e) => if e.kind != IoErrorKind.InvalidInput { return 12 }
    }
    match readText(data) { .Ok(t) => if t != "0123456789" { return 13 }, .Err(_) => return 14 }
    match (metadata(data), metadata(alias)) {
        (.Ok(a), .Ok(b)) => if not a.isSameFile(as: b) { return 15 },
        _ => return 16
    }
    // A real copy still works.
    match copy(data, to: dir / Path("copy.txt")) { .Ok(n) => if n != 10 { return 17 }, .Err(_) => return 18 }

    // A NUL byte in a path.
    match readText(Path("\(data.string)\0ignored")) {
        .Ok(_) => return 19,
        .Err(e) => if e.kind != IoErrorKind.InvalidInput { return 20 }
    }

    if Directory.removeAll(dir).isErr { return 21 }
    0
}
