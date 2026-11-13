// test: execution
// stdlib: true

// `Directory` create/entries/remove, `rename`, `copy`, symbolic links and
// permissions. `removeAll` deletes files inside it, which once named
// `Directory.remove` instead of the free `remove` and failed.

module Test

import std.streams.(IoErrorKind)

@main
func main() -> lang.i64 {
    let dir = match Directory.createTemporary(prefix: "kestrel-dirs") { .Ok(d) => d, .Err(_) => return 1 };
    let a = dir / Path("a.txt");
    if File.writeText(a, "data").isErr { return 2 }

    if File.rename(a, to: dir / Path("b.txt")).isErr { return 3 }
    if a.exists() { return 4 }
    match File.copy(dir / Path("b.txt"), to: dir / Path("c.txt")) {
        .Ok(n) => if n != 4 { return 5 },
        .Err(_) => return 6
    }
    if Directory.createAll(dir / Path("sub/deeper")).isErr { return 7 }
    // createAll on an existing directory is fine.
    if Directory.createAll(dir / Path("sub")).isErr { return 8 }

    var files = 0;
    var dirs = 0;
    match Directory.entries(dir) {
        .Ok(entries) => {
            for entry in entries {
                match entry {
                    .Ok(e) => if e.kind == FileKind.Directory { dirs = dirs + 1 } else { files = files + 1 },
                    .Err(_) => return 9
                }
            }
        },
        .Err(_) => return 10
    }
    if files != 2 or dirs != 1 { return 11 }

    // A non-empty directory can't be removed one level at a time.
    match Directory.remove(dir / Path("sub")) {
        .Ok(_) => return 12,
        .Err(e) => if e.kind != IoErrorKind.DirectoryNotEmpty { return 13 }
    }

    let link = dir / Path("link");
    if SymbolicLink.create(at: link, to: Path("b.txt")).isErr { return 14 }
    match SymbolicLink.read(link) { .Ok(t) => if t.string != "b.txt" { return 15 }, .Err(_) => return 16 }
    match Metadata.ofSymbolicLink(link) { .Ok(m) => if m.kind != FileKind.SymbolicLink { return 17 }, .Err(_) => return 18 }

    if File.setPermissions(dir / Path("c.txt"), Permissions(256)).isErr { return 19 }
    match Metadata.of(dir / Path("c.txt")) {
        .Ok(m) => if m.permissions.mode != 256 or not m.permissions.isReadOnly { return 20 },
        .Err(_) => return 21
    }

    match Directory.entries(Path("/definitely/not/a/real/dir-kestrel-test")) {
        .Ok(_) => return 22,
        .Err(e) => if e.kind != IoErrorKind.NotFound { return 23 }
    }

    if Directory.removeAll(dir).isErr { return 24 }
    if dir.exists() { return 25 }
    0
}
