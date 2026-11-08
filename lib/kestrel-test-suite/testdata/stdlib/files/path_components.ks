// test: execution
// stdlib: true

// `Path` lexical operations: name, stem, extension, parent, joining with
// `/`, components and normalizing. None of these touch the file system.

module Test

@main
func main() -> lang.i64 {
    let p: Path = "/usr/lib/archive.tar.gz";
    if p.name != .Some("archive.tar.gz") { return 1 }
    if p.stem != .Some("archive.tar") { return 2 }
    if p.extension != .Some("gz") { return 3 }
    if not p.isAbsolute { return 4 }
    match p.parent {
        .Some(parent) => if parent.string != "/usr/lib" { return 5 },
        .None => return 6
    }

    // A leading dot doesn't start an extension.
    let dotfile = Path(".profile");
    if dotfile.extension != .None { return 7 }
    if dotfile.stem != .Some(".profile") { return 8 }

    // The root has no parent or name.
    if Path("/").parent != .None { return 9 }
    if Path("/").name != .None { return 10 }

    let joined = Path("a/./b/../c") / Path("d.txt");
    if joined.normalized().string != "a/c/d.txt" { return 11 }
    // An absolute right side replaces the left.
    if (Path("a") / Path("/b")).string != "/b" { return 12 }

    let parts = Path("/a//b/./c/").components();
    if parts.count != 4 or parts(0) != "/" or parts(3) != "c" { return 13 }

    if Path("notes.txt").replacingExtension(with: "md").string != "notes.md" { return 14 }
    if "\(Path("x/y"))" != "x/y" { return 15 }
    0
}
