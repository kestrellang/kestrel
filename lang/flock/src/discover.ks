// Source file discovery
//
// Recursively finds all .ks files in a package directory.

module flock.discover

import flock.source.(joinPath)
import flock.error.(FlockError)
import flock.manifest.(BinDecl)

// ============================================================================
// BINARY TARGETS
// ============================================================================

/// A resolved binary target: the output `name` and the absolute `entry` source
/// file that supplies its `@main`.
public struct BinTarget: Cloneable {
    public var name: String
    public var entry: String

    public init(name name: String, entry entry: String) {
        self.name = name;
        self.entry = entry;
    }

    public func clone() -> BinTarget {
        BinTarget(name: self.name.clone(), entry: self.entry.clone())
    }
}

/// Discovers a package's binary targets, Cargo-style:
///   - `<src>/main.ks`        -> a bin named after the package
///   - `<src>/bin/*.ks`       -> one bin per direct child, named after the stem
///   - `[[bin]] { name, path }` -> overrides a convention bin of the same name,
///                                 or adds a new target (path relative to root)
///
/// Two targets resolving to the same name is a hard error (`DuplicateBinary`).
public func discoverBins(rootDir rootDir: String, sourceDir sourceDir: String, packageName packageName: String, bins bins: Array[BinDecl]) -> Array[BinTarget] throws FlockError {
    var result = [];
    let srcDir = joinPath(base: rootDir, rel: sourceDir);

    // src/main.ks -> default bin named after the package.
    let mainPath = joinPath(base: srcDir, rel: "main.ks");
    if fileExists(mainPath) {
        result.append(BinTarget(name: packageName, entry: mainPath))
    }

    // src/bin/*.ks -> one bin each (direct children only).
    let binDir = joinPath(base: srcDir, rel: "bin");
    if isDirectory(binDir) {
        let entries = listDir(binDir).unwrap(or: []);
        var i: Int64 = 0;
        while i < entries.count {
            let entry = entries(unchecked: i);
            i = i + 1;
            if entry.starts(with: ".") or not entry.ends(with: ".ks") {
                // skip hidden entries and non-.ks files
            } else {
                let full = joinPath(base: binDir, rel: entry);
                if not isDirectory(full) {
                    let stem = entry.asSlice().subslice(from: 0, to: entry.bytes.count - 3).toOwned();
                    result.append(BinTarget(name: stem, entry: full))
                }
            }
        }
    }

    // Apply [[bin]] (Cargo merge): a decl whose name matches a convention bin
    // overrides it; a new name adds a target. Rebuild to avoid subscript-set.
    if bins.count > 0 {
        var merged = [];
        var i: Int64 = 0;
        while i < result.count {
            let target = result(unchecked: i);
            i = i + 1;
            if not declsContainName(bins: bins, name: target.name) {
                merged.append(target)
            }
        }
        i = 0;
        while i < bins.count {
            let decl = bins(unchecked: i);
            i = i + 1;
            merged.append(BinTarget(name: decl.name, entry: joinPath(base: rootDir, rel: decl.path)))
        }
        result = merged
    }

    // Reject duplicate names (e.g. package `foo` with both src/main.ks and
    // src/bin/foo.ks, or two [[bin]] entries sharing a name).
    var i: Int64 = 0;
    while i < result.count {
        var j = i + 1;
        for element in result {
            if result(unchecked: i).name == element.name {
                throw FlockError.DuplicateBinary(result(unchecked: i).name)
            }
        }
        i = i + 1
    }

    .Ok(result)
}

/// True if any `[[bin]]` declaration carries the given name.
func declsContainName(bins bins: Array[BinDecl], name name: String) -> Bool {
    for element in bins {
        if element.name == name {
            return true
        }
    }
    false
}

// ============================================================================
// SOURCE DISCOVERY
// ============================================================================

/// Recursively discovers all .ks files in a package directory.
/// Skips hidden directories (starting with ".") and "target" directories.
public func discoverSources(rootDir rootDir: String) -> Array[String] {
    var result = [];
    let entries = listDir(rootDir).unwrap(or: []);
    var i: Int64 = 0;
    while i < entries.count {
        let entry = entries(unchecked: i);
        i = i + 1;

        // Skip hidden entries and target directory
        if entry.starts(with: ".") or entry == "target" {
            // skip
        } else {
            let fullPath = joinPath(base: rootDir, rel: entry);

            if isDirectory(fullPath) {
                // Recurse into subdirectories
                let subFiles = discoverSources(rootDir: fullPath);
                for element in subFiles {
                    result.append(element);
                }
            } else if entry.ends(with: ".ks") {
                result.append(fullPath)
            }
        }
    }
    result
}
