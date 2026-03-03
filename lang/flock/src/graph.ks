// Dependency graph construction and topological sort

module flock.graph

import flock.error.(FlockError)
import flock.source.(ResolvedPackage, PathSource)
import flock.dependency.(Dependency, DependencySpec)
import flock.manifest.(BuildConfig)
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

    public init(name name: String, rootDir rootDir: String, sourceDir sourceDir: String, depNames depNames: Array[String], build build: BuildConfig) {
        self.name = name;
        self.rootDir = rootDir;
        self.sourceDir = sourceDir;
        self.depNames = depNames;
        self.build = build;
    }

    public func clone() -> DepNode {
        DepNode(name: self.name.clone(), rootDir: self.rootDir.clone(), sourceDir: self.sourceDir.clone(), depNames: self.depNames.clone(), build: self.build.clone())
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
) -> Result[Array[DepNode], FlockError] {
    var nodes = Array[DepNode]();
    var visited = Array[String]();
    var queue = Array[ResolvedPackage]();

    queue.append(root);
    visited.append(root.name);

    while queue.count > 0 {
        let current = queue(unchecked: 0);
        queue = sliceFrom(queue, 1);

        // Collect dependency names for this node
        var depNames = Array[String]();
        let deps = current.manifest.dependencies;
        var i: Int64 = 0;
        while i < deps.count {
            let dep = deps(unchecked: i);
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
                    .Err(e) => return .Err(e)
                }
            }
            i = i + 1
        }

        nodes.append(DepNode(
            name: current.name,
            rootDir: current.rootDir,
            sourceDir: current.manifest.package.source,
            depNames: depNames,
            build: current.manifest.build
        ))
    }

    .Ok(nodes)
}

// ============================================================================
// TOPOLOGICAL SORT
// ============================================================================

/// Sorts dependency nodes in build order (dependencies before dependents).
/// Returns an error if a cycle is detected.
public func topologicalSort(nodes nodes: Array[DepNode]) -> Result[Array[DepNode], FlockError] {
    let count = nodes.count;
    if count == 0 {
        return .Ok(Array[DepNode]())
    }

    // Compute in-degrees
    var inDegrees = Array[Int64]();
    var i: Int64 = 0;
    while i < count {
        inDegrees.append(0);
        i = i + 1
    }

    i = 0;
    while i < count {
        let node = nodes(unchecked: i);
        var j: Int64 = 0;
        while j < node.depNames.count {
            let depName = node.depNames(unchecked: j);
            match findIndex(nodes: nodes, name: depName) {
                .Some(idx) => {
                    // The dependency (idx) is depended on by node (i)
                    // But in-degree tracks how many deps each node has
                },
                .None => {} // External dep, ignore
            }
            j = j + 1
        }
        // In-degree = number of deps that are in the graph
        var depCount: Int64 = 0;
        j = 0;
        while j < node.depNames.count {
            let depName = node.depNames(unchecked: j);
            if containsNode(nodes: nodes, name: depName) {
                depCount = depCount + 1
            }
            j = j + 1
        }
        inDegrees = setAt(arr: inDegrees, index: i, value: depCount);
        i = i + 1
    }

    // Kahn's algorithm: process nodes with 0 in-degree
    var result = Array[DepNode]();
    var processed: Int64 = 0;

    while processed < count {
        // Find a node with in-degree 0
        var found: Int64 = -1;
        i = 0;
        while i < count {
            if inDegrees(unchecked: i) == 0 {
                // Check it hasn't been added already
                if not containsNode(nodes: result, name: nodes(unchecked: i).name) {
                    found = i;
                    break
                }
            }
            i = i + 1
        }

        if found < 0 {
            // Cycle detected — collect remaining node names
            var cycleNames = Array[String]();
            i = 0;
            while i < count {
                if not containsNode(nodes: result, name: nodes(unchecked: i).name) {
                    cycleNames.append(nodes(unchecked: i).name)
                }
                i = i + 1
            }
            return .Err(FlockError.DependencyCycle(cycleNames))
        }

        let node = nodes(unchecked: found);
        result.append(node);
        // Mark as done by setting in-degree to -1
        inDegrees = setAt(arr: inDegrees, index: found, value: -1);

        // Decrease in-degree for nodes that depend on this one
        i = 0;
        while i < count {
            if inDegrees(unchecked: i) > 0 {
                let otherNode = nodes(unchecked: i);
                if containsInDeps(depNames: otherNode.depNames, name: node.name) {
                    inDegrees = setAt(arr: inDegrees, index: i, value: inDegrees(unchecked: i) - 1)
                }
            }
            i = i + 1
        }

        processed = processed + 1
    }

    .Ok(result)
}

// ============================================================================
// HELPERS
// ============================================================================

func contains(arr arr: Array[String], value value: String) -> Bool {
    var i: Int64 = 0;
    while i < arr.count {
        if arr(unchecked: i).equals(value) {
            return true
        }
        i = i + 1
    }
    false
}

func containsNode(nodes nodes: Array[DepNode], name name: String) -> Bool {
    var i: Int64 = 0;
    while i < nodes.count {
        if nodes(unchecked: i).name.equals(name) {
            return true
        }
        i = i + 1
    }
    false
}

func containsInDeps(depNames depNames: Array[String], name name: String) -> Bool {
    contains(arr: depNames, value: name)
}

func findIndex(nodes nodes: Array[DepNode], name name: String) -> Optional[Int64] {
    var i: Int64 = 0;
    while i < nodes.count {
        if nodes(unchecked: i).name.equals(name) {
            return .Some(i)
        }
        i = i + 1
    }
    .None
}

func sliceFrom(arr: Array[ResolvedPackage], start: Int64) -> Array[ResolvedPackage] {
    var result = Array[ResolvedPackage]();
    var i = start;
    while i < arr.count {
        result.append(arr(unchecked: i));
        i = i + 1
    }
    result
}

func setAt(arr arr: Array[Int64], index index: Int64, value value: Int64) -> Array[Int64] {
    var result = Array[Int64]();
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
