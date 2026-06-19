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
public func discoverBins(rootDir rootDir: String, sourceDir sourceDir: String, packageName packageName: String, bins bins: Array[BinDecl]) -> Result[Array[BinTarget], FlockError] {
    var result = Array[BinTarget]();
    let srcDir = joinPath(base: rootDir, rel: sourceDir);

    // src/main.ks -> default bin named after the package.
    let mainPath = joinPath(base: srcDir, rel: "main.ks");
    if fileExists(mainPath) {
        result.append(BinTarget(name: packageName, entry: mainPath))
    }

    // src/bin/*.ks -> one bin each (direct children only).
    let binDir = joinPath(base: srcDir, rel: "bin");
    if isDirectory(binDir) {
        let entries = listDir(binDir);
        var i: Int64 = 0;
        while i < entries.count {
            let entry = entries(unchecked: i);
            i = i + 1;
            if entry.starts(with: ".") or not entry.ends(with: ".ks") {
                // skip hidden entries and non-.ks files
            } else {
                let full = joinPath(base: binDir, rel: entry);
                if not isDirectory(full) {
                    let stem = entry.asSlice().subslice(from: 0, to: entry.byteCount - 3).toOwned();
                    result.append(BinTarget(name: stem, entry: full))
                }
            }
        }
    }

    // Apply [[bin]] (Cargo merge): a decl whose name matches a convention bin
    // overrides it; a new name adds a target. Rebuild to avoid subscript-set.
    if bins.count > 0 {
        var merged = Array[BinTarget]();
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
        while j < result.count {
            if result(unchecked: i).name == result(unchecked: j).name {
                return .Err(FlockError.DuplicateBinary(result(unchecked: i).name))
            }
            j = j + 1
        }
        i = i + 1
    }

    .Ok(result)
}

/// True if any `[[bin]]` declaration carries the given name.
func declsContainName(bins bins: Array[BinDecl], name name: String) -> Bool {
    var i: Int64 = 0;
    while i < bins.count {
        if bins(unchecked: i).name == name {
            return true
        }
        i = i + 1
    }
    false
}

// ============================================================================
// SOURCE DISCOVERY
// ============================================================================

/// Recursively discovers all .ks files in a package directory.
/// Skips hidden directories (starting with ".") and "target" directories.
public func discoverSources(rootDir rootDir: String) -> Array[String] {
    var result = Array[String]();
    let entries = listDir(rootDir);
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
                var j: Int64 = 0;
                while j < subFiles.count {
                    result.append(subFiles(unchecked: j));
                    j = j + 1
                }
            } else if entry.ends(with: ".ks") {
                result.append(fullPath)
            }
        }
    }
    result
}
