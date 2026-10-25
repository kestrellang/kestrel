// Dependency graph construction and topological sort

module flock.graph

import flock.error.(FlockError)
import flock.source.(ResolvedPackage, PathSource)
import flock.dependency.(Dependency, DependencySpec)
import flock.manifest.(BuildConfig, BinDecl)
import flock.registry_source.(RegistrySource)

// ============================================================================
// DEPENDENCY NODE
// ============================================================================

/// A node in the dependency graph.
public struct DepNode: Cloneable {
    public var name: String
    public var rootDir: String
    /// Source directory relative to rootDir (e.g. "src").
    public var sourceDir: String
    public var depNames: Array[String]
    /// Build configuration (C sources, link flags, etc.).
    public var build: BuildConfig
    /// `[[bin]]` declarations for this package, carried so per-package binary
    /// entries can be excluded from the shared compile (a dependency's own
    /// bin entries must never be pulled into a dependent's build).
    public var bins: Array[BinDecl]

    public init(name name: String, rootDir rootDir: String, sourceDir sourceDir: String, depNames depNames: Array[String], build build: BuildConfig, bins bins: Array[BinDecl]) {
        self.name = name;
        self.rootDir = rootDir;
        self.sourceDir = sourceDir;
        self.depNames = depNames;
        self.build = build;
        self.bins = bins;
    }

    public func clone() -> DepNode {
        DepNode(name: self.name.clone(), rootDir: self.rootDir.clone(), sourceDir: self.sourceDir.clone(), depNames: self.depNames.clone(), build: self.build.clone(), bins: self.bins.clone())
    }
}

// ============================================================================
// GRAPH CONSTRUCTION
// ============================================================================

/// Builds a dependency graph starting from the root package.
/// Uses BFS to resolve all transitive dependencies.
/// Dispatches to PathSource or RegistrySource based on the dependency spec.
public func buildGraph(
    root root: ResolvedPackage,
    pathSource pathSource: PathSource,
    registrySource registrySource: RegistrySource
) -> Array[DepNode] throws FlockError {
    var nodes = [];
    var visited = [];
    var queue = [];

    queue.append(root);
    visited.append(root.name);

    while queue.count > 0 {
        let current = queue(unchecked: 0);
        queue = sliceFrom(queue, 1);

        // Collect dependency names for this node
        var depNames = [];
        let deps = current.manifest.dependencies;
        for dep in deps {
            depNames.append(dep.name);

            // Resolve and enqueue if not yet visited
            if not contains(arr: visited, value: dep.name) {
                visited.append(dep.name);
                let resolveResult = match dep.spec {
                    .Path(_) => pathSource.resolve(name: dep.name, spec: dep.spec, baseDir: current.rootDir),
                    .Registry(_) => registrySource.resolve(name: dep.name, spec: dep.spec, baseDir: current.rootDir)
                };
                match resolveResult {
                    .Ok(resolved) => queue.append(resolved),
                    .Err(e) => throw e
                }
            }
        }

        nodes.append(DepNode(
            name: current.name,
            rootDir: current.rootDir,
            sourceDir: current.manifest.package.source,
            depNames: depNames,
            build: current.manifest.build,
            bins: current.manifest.bins
        ))
    }

    .Ok(nodes)
}

// ============================================================================
// TOPOLOGICAL SORT
// ============================================================================

/// Sorts dependency nodes in build order (dependencies before dependents).
/// Returns an error if a cycle is detected.
public func topologicalSort(nodes nodes: Array[DepNode]) -> Array[DepNode] throws FlockError {
    let count = nodes.count;
    if count == 0 {
        return .Ok([])
    }

    // Compute in-degrees
    var inDegrees: [Int64] = [];
    for _ in 0..<count {
        inDegrees.append(0)
    }

    // In-degree = how many of a node's deps are themselves in the graph.
    for (index, node) in nodes.iter().enumerate() {
        var depCount: Int64 = 0;
        for depName in node.depNames {
            if containsNode(nodes: nodes, name: depName) {
                depCount = depCount + 1
            }
        }
        inDegrees = setAt(arr: inDegrees, index: index, value: depCount)
    }

    // Kahn's algorithm: process nodes with 0 in-degree
    var result: [DepNode] = [];
    var processed: Int64 = 0;

    while processed < count {
        // Find a node with in-degree 0 that hasn't been emitted yet
        var found: Int64 = -1;
        for index in 0..<count {
            if inDegrees(unchecked: index) != 0 { continue }
            if containsNode(nodes: result, name: nodes(unchecked: index).name) { continue }
            found = index;
            break
        }

        if found < 0 {
            // Cycle detected — collect remaining node names
            var cycleNames: [String] = [];
            for node in nodes {
                if not containsNode(nodes: result, name: node.name) {
                    cycleNames.append(node.name)
                }
            }
            throw FlockError.DependencyCycle(cycleNames)
        }

        let node = nodes(unchecked: found);
        result.append(node);
        // Mark as done by setting in-degree to -1
        inDegrees = setAt(arr: inDegrees, index: found, value: -1);

        // Decrease in-degree for nodes that depend on this one. Indexed rather
        // than iterated because `setAt` rebuilds `inDegrees` each time.
        for index in 0..<count {
            let degree = inDegrees(unchecked: index);
            if degree <= 0 { continue }
            let otherNode = nodes(unchecked: index);
            if containsInDeps(depNames: otherNode.depNames, name: node.name) {
                inDegrees = setAt(arr: inDegrees, index: index, value: degree - 1)
            }
        }

        processed = processed + 1
    }

    .Ok(result)
}

// ============================================================================
// HELPERS
// ============================================================================

func contains(arr arr: Array[String], value value: String) -> Bool {
    for element in arr {
        if element == value {
            return true
        }
    }
    false
}

func containsNode(nodes nodes: Array[DepNode], name name: String) -> Bool {
    for element in nodes {
        if element.name == name {
            return true
        }
    }
    false
}

func containsInDeps(depNames depNames: Array[String], name name: String) -> Bool {
    contains(arr: depNames, value: name)
}

func findIndex(nodes nodes: Array[DepNode], name name: String) -> Int64? {
    var i: Int64 = 0;
    while i < nodes.count {
        if nodes(unchecked: i).name == name {
            return .Some(i)
        }
        i = i + 1
    }
    .None
}

func sliceFrom(arr: Array[ResolvedPackage], start: Int64) -> Array[ResolvedPackage] {
    var result = [];
    var i = start;
    for element in arr {
        result.append(element);
    }
    result
}

func setAt(arr arr: Array[Int64], index index: Int64, value value: Int64) -> Array[Int64] {
    var result = [];
    var i: Int64 = 0;
    while i < arr.count {
        if i == index {
            result.append(value)
        } else {
            result.append(arr(unchecked: i))
        }
        i = i + 1
    }
    result
}
