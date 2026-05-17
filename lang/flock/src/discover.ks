// Source file discovery
//
// Recursively finds all .ks files in a package directory.

module flock.discover

import flock.source.(joinPath)

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
