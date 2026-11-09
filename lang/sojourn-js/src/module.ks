module js

// ECMAScript modules (§16.2): loading a module graph from files, linking
// imports to exports, evaluating, namespace objects, and import().
//
// A module's top-level bindings live in one environment, allocated when
// the graph links. Its code is two arrows compiled over that scope (see
// compile_module.ks): `init` creates its functions and puts its lexical
// bindings in the TDZ, and `body` runs the statements (as an async arrow
// when the module has top-level await). An import binding's slot holds
// an import reference (`importRefValue`) to the exporting module's slot,
// which GetVar follows, so imports stay live.

import std.files.(File, Path)
import std.collections.(Dictionary, Set)

/// `import x from "m"` and friends: `importName` "*" is a namespace import.
struct ImportEntry {
    var request: String
    var importName: String
    var localName: String
}

/// One export. Local: `request` "" and `localName` the binding.
/// Indirect: `request` and `importName` ("*" for `export * as ns`).
/// Star (`export * from`): `exportName` "".
struct ExportEntry {
    var exportName: String
    var request: String
    var importName: String
    var localName: String
}

/// Module status.
struct ModuleStatus {
    static var new: Int64 { 0 }
    static var linked: Int64 { 1 }
    static var evaluating: Int64 { 2 }
    /// Waiting on its own top-level await or an async dependency.
    static var evaluatingAsync: Int64 { 3 }
    static var evaluated: Int64 { 4 }
}

struct ModuleRecord {
    var path: String
    var status: Int64
    /// Specifiers in source order, and the modules they load.
    var requests: [String]
    var requested: [Int64]
    var imports: [ImportEntry]
    var localExports: [ExportEntry]
    var indirectExports: [ExportEntry]
    var starExports: [ExportEntry]
    /// The module scope's slots by name, and its size.
    var slots: [String: Int64]
    var envSize: Int64
    var env: Int64
    var initCode: Int64
    var bodyCode: Int64
    var hasTopLevelAwait: Bool
    var namespace: Value
    var meta: Value
    var error: Value
    var hasError: Bool
    /// Settles when evaluation finishes (async graphs), else undefined.
    var promise: Value
    /// Async evaluation: dependencies still running, and the modules
    /// waiting on this one.
    var pendingDeps: Int64
    var asyncParents: [Int64]
    var isAsyncEvaluation: Bool
    /// Order in which async modules finished their dependencies.
    var asyncOrder: Int64
    /// Tarjan bookkeeping for cycles.
    var dfsIndex: Int64
    var dfsAncestor: Int64
    var cycleRoot: Int64
}

/// A cross-module binding: slot `slot` of environment `env`.
struct ImportRef {
    var env: Int64
    var slot: Int64
}

/// ResolveExport's answer: not found, ambiguous, or a binding (`name`
/// "*namespace*" for the module's namespace object).
struct ExportResolution {
    var kind: Int64
    var owner: Int64
    var name: String
}

func resolutionNotFound() -> ExportResolution { ExportResolution(kind: 0, owner: -1, name: "") }
func resolutionAmbiguous() -> ExportResolution { ExportResolution(kind: 1, owner: -1, name: "") }

/// An empty value whose payload names an ImportRef (index + 1). Payload 0
/// is the ordinary TDZ marker.
func importRefValue(index: Int64) -> Value { Value.boxed(Tag.empty, index + 1) }

func isImportRef(v: Value) -> Bool { v.isEmpty and v.payload > 0 }

// ----------------------------------------------------------------------
// Paths
// ----------------------------------------------------------------------

func pathSegments(s: String) -> Array[String] {
    var out: [String] = [];
    var cur = "";
    for c in s.chars {
        if c == '/' {
            out.append(cur);
            cur = "";
        } else {
            cur.append(char: c);
        }
    }
    out.append(cur);
    out
}

/// `spec` relative to the directory of `referrer`, with `.` and `..`
/// folded away.
func resolveModulePath(referrer: String, spec: String) -> String {
    var parts: [String] = [];
    let absolute = spec.starts(with: "/");
    if not absolute {
        let base = pathSegments(referrer);
        for i in 0..<(base.count - 1) { parts.append(base(i)); }
    }
    for seg in pathSegments(spec) { parts.append(seg); }
    var out: [String] = [];
    var first = true;
    for seg in parts {
        if seg == "." { continue; }
        if seg == "" and not first { continue; }
        first = false;
        if seg == ".." and out.count > 0 and out(out.count - 1) != ".." and out(out.count - 1) != "" {
            let _ = out.pop();
            continue;
        }
        out.append(seg);
    }
    var s = "";
    for i in 0..<out.count {
        if i > 0 { s.append("/"); }
        s.append(out(i));
    }
    if s.isEmpty { "." } else { s }
}

extend VM {
    // ------------------------------------------------------------------
    // Loading
    // ------------------------------------------------------------------

    /// Loads `path` and everything it imports, compiling each module once.
    mutating func loadModule(path: String) -> Result[Int64, Value] {
        if let .Some(m) = self.moduleByPath(path) { return .Ok(m); }
        let source = match File.readText(Path(path)) {
            .Ok(s) => s,
            .Err(_) => return .Err(self.makeError(.SyntaxError, "Cannot load module '\(path)'"))
        };
        let program = match parseModule(source) {
            .Ok(p) => p,
            .Err(e) => return .Err(self.makeError(.SyntaxError, e.message))
        };
        let index = self.modules.count;
        var rec = ModuleRecord(path: path, status: ModuleStatus.new, requests: [], requested: [], imports: [],
                               localExports: [], indirectExports: [], starExports: [], slots: [:], envSize: 0,
                               env: -1, initCode: -1, bodyCode: -1, hasTopLevelAwait: program.hasTopLevelAwait,
                               namespace: Value.undefined, meta: Value.undefined, error: Value.undefined,
                               hasError: false, promise: Value.undefined, pendingDeps: 0, asyncParents: [],
                               isAsyncEvaluation: false, asyncOrder: 0, dfsIndex: -1, dfsAncestor: -1, cycleRoot: -1);
        collectModuleEntries(program, rec);
        self.modules.append(rec);
        let _ = self.moduleIndex.insert(path, index);
        let saved = self.sourceName;
        self.sourceName = path;
        let compiled = self.compileModule(program, index);
        self.sourceName = saved;
        match compiled {
            .Ok(_) => {},
            .Err(e) => {
                self.fns = [];
                return .Err(self.makeError(.SyntaxError, e.message));
            }
        }
        let requests = self.modules(index).requests;
        for spec in requests {
            let dep = try self.loadModule(resolveModulePath(path, spec));
            self.modules(index).requested.append(dep);
        }
        .Ok(index)
    }

    func moduleByPath(path: String) -> Optional[Int64] { self.moduleIndex(path) }

    /// The module `m` loads for `spec`.
    func requestedModule(m: Int64, spec: String) -> Int64 {
        let rec = self.modules(m);
        for i in 0..<rec.requests.count {
            if rec.requests(i) == spec { return rec.requested(i); }
        }
        return -1;
    }

    // ------------------------------------------------------------------
    // Linking
    // ------------------------------------------------------------------

    /// Links the graph under `root`: every module gets its environment,
    /// its imports point at their exports, and its functions exist.
    mutating func linkModule(root: Int64) -> Result[(), Value] {
        var graph: [Int64] = [];
        self.moduleGraph(root, graph);
        var fresh: [Int64] = [];
        for m in graph {
            if self.modules(m).status != ModuleStatus.new { continue; }
            fresh.append(m);
            var slots: [Value] = [];
            for _ in 0..<self.modules(m).envSize { slots.append(Value.undefined); }
            self.modules(m).env = self.heap.allocEnv(Env(slots: slots, parent: self.globalEnv));
        }
        for m in fresh { try self.resolveImports(m); }
        for m in fresh {
            self.modules(m).status = ModuleStatus.linked;
            let f = self.makeClosureObject(self.modules(m).initCode, self.modules(m).env, Value.undefined);
            let _ = try self.callValue(f, Value.undefined, []);
        }
        .Ok(())
    }

    /// Every module reachable from `m`, dependencies first.
    func moduleGraph(m: Int64, mutating out: Array[Int64]) {
        if out.contains(m) { return; }
        out.append(m);
        for d in self.modules(m).requested { self.moduleGraph(d, out); }
    }

    /// InitializeEnvironment's import half, plus the check that every
    /// indirect export resolves.
    mutating func resolveImports(m: Int64) -> Result[(), Value] {
        let rec = self.modules(m);
        for e in rec.indirectExports {
            let r = self.resolveExport(m, e.exportName, []);
            if r.kind != 2 { return .Err(self.makeError(.SyntaxError, "The requested module does not provide an export named '\(e.exportName)'")); }
        }
        for imp in rec.imports {
            let target = self.requestedModule(m, imp.request);
            let slot = rec.slots(imp.localName) ?? -1;
            if imp.importName == "*" {
                let ns = self.moduleNamespace(target);
                self.heap.envs(rec.env).slots(slot) = ns;
                continue;
            }
            let r = self.resolveExport(target, imp.importName, []);
            if r.kind == 0 { return .Err(self.makeError(.SyntaxError, "The requested module '\(imp.request)' does not provide an export named '\(imp.importName)'")); }
            if r.kind == 1 { return .Err(self.makeError(.SyntaxError, "The requested module '\(imp.request)' contains conflicting star exports for name '\(imp.importName)'")); }
            self.heap.envs(rec.env).slots(slot) = self.bindingValue(r);
        }
        .Ok(())
    }

    /// What a slot bound to resolution `r` holds: the namespace object,
    /// or a reference to the exporting slot.
    mutating func bindingValue(r: ExportResolution) -> Value {
        if r.name == "*namespace*" { return self.moduleNamespace(r.owner); }
        let slot = self.modules(r.owner).slots(r.name) ?? -1;
        self.importRefs.append(ImportRef(env: self.modules(r.owner).env, slot: slot));
        importRefValue(self.importRefs.count - 1)
    }

    /// ResolveExport(module, exportName, resolveSet).
    func resolveExport(m: Int64, name: String, seen: Array[(Int64, String)]) -> ExportResolution {
        for (sm, sn) in seen {
            if sm == m and sn == name { return resolutionNotFound(); }
        }
        var visited = seen;
        visited.append((m, name));
        let rec = self.modules(m);
        for e in rec.localExports {
            if e.exportName != name { continue; }
            // A re-exported `import * as ns` is the imported module's namespace.
            for imp in rec.imports {
                if imp.localName == e.localName and imp.importName == "*" {
                    return ExportResolution(kind: 2, owner: self.requestedModule(m, imp.request), name: "*namespace*");
                }
            }
            return ExportResolution(kind: 2, owner: m, name: e.localName);
        }
        for e in rec.indirectExports {
            if e.exportName != name { continue; }
            let imported = self.requestedModule(m, e.request);
            if e.importName == "*" { return ExportResolution(kind: 2, owner: imported, name: "*namespace*"); }
            return self.resolveExport(imported, e.importName, visited);
        }
        if name == "default" { return resolutionNotFound(); }
        var star = resolutionNotFound();
        for e in rec.starExports {
            let imported = self.requestedModule(m, e.request);
            let r = self.resolveExport(imported, name, visited);
            if r.kind == 1 { return r; }
            if r.kind == 0 { continue; }
            if star.kind == 0 {
                star = r;
            } else if star.owner != r.owner or star.name != r.name {
                return resolutionAmbiguous();
            }
        }
        star
    }

    /// GetExportedNames(module, exportStarSet).
    func exportedNames(m: Int64, mutating seen: Array[Int64]) -> Array[String] {
        if seen.contains(m) { return []; }
        seen.append(m);
        let rec = self.modules(m);
        var names: [String] = [];
        for e in rec.localExports { names.append(e.exportName); }
        for e in rec.indirectExports { names.append(e.exportName); }
        for e in rec.starExports {
            let imported = self.requestedModule(m, e.request);
            for n in self.exportedNames(imported, seen) {
                if n != "default" and not names.contains(n) { names.append(n); }
            }
        }
        names
    }

    // ------------------------------------------------------------------
    // Namespaces
    // ------------------------------------------------------------------

    /// GetModuleNamespace: made once, with the unambiguous export names
    /// sorted by code units.
    mutating func moduleNamespace(m: Int64) -> Value {
        if self.modules(m).namespace.isObject { return self.modules(m).namespace; }
        let ns = self.newObjectKind(Value.nullValue, .ModuleNamespace(index: m));
        self.modules(m).namespace = ns;
        var seen: [Int64] = [];
        var names: [Units] = [];
        for n in self.exportedNames(m, seen) {
            if self.resolveExport(m, n, []).kind == 2 { names.append(unitsOf(n)); }
        }
        // Insertion sort: export lists are short.
        var i: Int64 = 1;
        while i < names.count {
            let x = names(i);
            var j = i - 1;
            while j >= 0 and unitsLess(x, names(j)) {
                names(j + 1) = names(j);
                j = j - 1;
            }
            names(j + 1) = x;
            i = i + 1;
        }
        for u in names {
            let r = self.resolveExport(m, textOfUnits(u), []);
            let v = self.bindingValue(r);
            self.defineOwn(ns.asRef, self.atomOfUnits(u), v, Attr.writable | Attr.enumerable);
        }
        let tag = self.str("Module");
        self.defineOwn(ns.asRef, symbolKey(WellKnown.toStringTag), tag, 0);
        self.heap.objects(ns.asRef).extensible = false;
        ns
    }

    func isNamespace(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .ModuleNamespace(_) => true,
            _ => false
        }
    }

    /// A namespace's [[Get]]: an export's live value (a ReferenceError
    /// while it's uninitialized), its @@toStringTag, or undefined.
    mutating func namespaceGet(r: Ref, key: Key) -> Result[Value, Value] {
        match self.ownProperty(r, key) {
            .Some(p) => {
                if p.value.isEmpty { return .Err(self.makeError(.ReferenceError, "Cannot access '\(self.keyText(key))' before initialization")); }
                .Ok(p.value)
            },
            .None => .Ok(Value.undefined)
        }
    }

    /// A namespace's [[DefineOwnProperty]] for a string key: true only
    /// for a descriptor its export already matches.
    mutating func namespaceDefine(r: Ref, key: Key, d: Descriptor) -> Result[Bool, Value] {
        let current = match self.ownProperty(r, key) {
            .Some(p) => p,
            .None => return .Ok(false)
        };
        if current.value.isEmpty { return .Err(self.makeError(.ReferenceError, "Cannot access '\(self.keyText(key))' before initialization")); }
        if (d.configurable ?? false) or not (d.enumerable ?? true) { return .Ok(false); }
        if isAccessorDescriptor(d) or not (d.writable ?? true) { return .Ok(false); }
        if let .Some(v) = d.value { return .Ok(self.sameValue(v, current.value)); }
        .Ok(true)
    }

    /// The live value behind an import reference (maybe the TDZ marker).
    func derefImport(v: Value) -> Value {
        if not isImportRef(v) { return v; }
        let r = self.importRefs(v.payload - 1);
        self.heap.envs(r.env).slots(r.slot)
    }

    // ------------------------------------------------------------------
    // Evaluation
    // ------------------------------------------------------------------

    /// Evaluates the graph under `root` (already linked). Returns a promise
    /// for its completion when anything in it is async, else undefined.
    mutating func evaluateModule(root: Int64) -> Result[Value, Value] {
        var stack: [Int64] = [];
        var index: Int64 = 0;
        if let .Err(e) = self.innerModuleEvaluation(root, stack, index) {
            for m in stack {
                self.modules(m).status = ModuleStatus.evaluated;
                self.modules(m).hasError = true;
                self.modules(m).error = e;
            }
            return .Err(e);
        }
        let rootRec = self.modules(self.cycleRootOf(root));
        if rootRec.hasError { return .Err(rootRec.error); }
        .Ok(rootRec.promise)
    }

    func cycleRootOf(m: Int64) -> Int64 {
        let r = self.modules(m).cycleRoot;
        if r < 0 { m } else { r }
    }

    /// InnerModuleEvaluation: Tarjan's algorithm over the graph, running
    /// each strongly connected component once its dependencies have.
    mutating func innerModuleEvaluation(m: Int64, mutating stack: Array[Int64], mutating index: Int64) -> Result[Int64, Value] {
        let st = self.modules(m).status;
        if st == ModuleStatus.evaluatingAsync or st == ModuleStatus.evaluated {
            let root = self.cycleRootOf(m);
            if self.modules(root).hasError { return .Err(self.modules(root).error); }
            return .Ok(index);
        }
        if st == ModuleStatus.evaluating { return .Ok(index); }
        self.modules(m).status = ModuleStatus.evaluating;
        self.modules(m).dfsIndex = index;
        self.modules(m).dfsAncestor = index;
        self.modules(m).pendingDeps = 0;
        var idx = index + 1;
        stack.append(m);
        let deps = self.modules(m).requested;
        for d in deps {
            idx = try self.innerModuleEvaluation(d, stack, idx);
            var req = d;
            if self.modules(d).status == ModuleStatus.evaluating {
                let a = if self.modules(m).dfsAncestor < self.modules(d).dfsAncestor { self.modules(m).dfsAncestor } else { self.modules(d).dfsAncestor };
                self.modules(m).dfsAncestor = a;
            } else {
                req = self.cycleRootOf(d);
                if self.modules(req).hasError { return .Err(self.modules(req).error); }
            }
            if self.modules(req).isAsyncEvaluation {
                self.modules(m).pendingDeps = self.modules(m).pendingDeps + 1;
                if not self.modules(req).asyncParents.contains(m) { self.modules(req).asyncParents.append(m); }
            }
        }
        if self.modules(m).pendingDeps > 0 or self.modules(m).hasTopLevelAwait {
            self.modules(m).isAsyncEvaluation = true;
            self.asyncEvaluationCount = self.asyncEvaluationCount + 1;
            self.modules(m).asyncOrder = self.asyncEvaluationCount;
            if self.modules(m).pendingDeps == 0 { self.executeAsyncModule(m); }
        } else {
            try self.executeModule(m);
        }
        if self.modules(m).dfsAncestor == self.modules(m).dfsIndex {
            while true {
                let r = stack.pop() ?? m;
                self.modules(r).status = if self.modules(r).isAsyncEvaluation { ModuleStatus.evaluatingAsync } else { ModuleStatus.evaluated };
                self.modules(r).cycleRoot = m;
                if r == m { break; }
            }
        }
        .Ok(idx)
    }

    /// Runs a synchronous module's body.
    mutating func executeModule(m: Int64) -> Result[(), Value] {
        let f = self.makeClosureObject(self.modules(m).bodyCode, self.modules(m).env, Value.undefined);
        let _ = try self.callValue(f, Value.undefined, []);
        .Ok(())
    }

    /// Starts an async module's body; its promise reports back through
    /// `asyncModuleSettled`.
    mutating func executeAsyncModule(m: Int64) {
        let f = self.makeClosureObject(self.modules(m).bodyCode, self.modules(m).env, Value.undefined);
        self.pushRoot(f);
        let result = self.callValue(f, Value.undefined, []);
        let p = match result {
            .Ok(v) => v,
            .Err(e) => {
                self.asyncModuleRejected(m, e);
                return;
            }
        };
        if not self.modules(m).hasTopLevelAwait {
            self.asyncModuleFulfilled(m);
            return;
        }
        self.pushRoot(p);
        let onF = self.nativeClosure(.ModuleAsyncFulfilled, 1, [Value.int(m)]);
        self.pushRoot(onF);
        let onR = self.nativeClosure(.ModuleAsyncRejected, 1, [Value.int(m)]);
        self.pushRoot(onR);
        self.performThen(p, onF, onR, Value.undefined, Value.undefined, Value.undefined);
    }

    /// AsyncModuleExecutionFulfilled.
    mutating func asyncModuleFulfilled(m: Int64) {
        if self.modules(m).status == ModuleStatus.evaluated { return; }
        self.modules(m).isAsyncEvaluation = false;
        self.modules(m).status = ModuleStatus.evaluated;
        self.settleModulePromise(m, false, Value.undefined);
        // Parents whose last pending dependency this was run, in the
        // order they started waiting.
        var ready: [Int64] = [];
        self.gatherReadyParents(m, ready);
        var sorted = ready;
        var i: Int64 = 1;
        while i < sorted.count {
            let x = sorted(i);
            var j = i - 1;
            while j >= 0 and self.modules(sorted(j)).asyncOrder > self.modules(x).asyncOrder {
                sorted(j + 1) = sorted(j);
                j = j - 1;
            }
            sorted(j + 1) = x;
            i = i + 1;
        }
        for p in sorted {
            if self.modules(p).status == ModuleStatus.evaluated and self.modules(p).hasError { continue; }
            if self.modules(p).hasTopLevelAwait {
                self.executeAsyncModule(p);
                continue;
            }
            match self.executeModule(p) {
                .Ok(_) => {
                    self.modules(p).isAsyncEvaluation = false;
                    self.modules(p).status = ModuleStatus.evaluated;
                    self.settleModulePromise(p, false, Value.undefined);
                },
                .Err(e) => self.asyncModuleRejected(p, e)
            }
        }
    }

    /// GatherAvailableAncestors.
    mutating func gatherReadyParents(m: Int64, mutating out: Array[Int64]) {
        for p in self.modules(m).asyncParents {
            if out.contains(p) { continue; }
            let root = self.cycleRootOf(p);
            if self.modules(root).hasError { continue; }
            self.modules(p).pendingDeps = self.modules(p).pendingDeps - 1;
            if self.modules(p).pendingDeps == 0 {
                out.append(p);
                if not self.modules(p).hasTopLevelAwait { self.gatherReadyParents(p, out); }
            }
        }
    }

    /// AsyncModuleExecutionRejected.
    mutating func asyncModuleRejected(m: Int64, e: Value) {
        if self.modules(m).status == ModuleStatus.evaluated and self.modules(m).hasError { return; }
        self.modules(m).hasError = true;
        self.modules(m).error = e;
        self.modules(m).isAsyncEvaluation = false;
        self.modules(m).status = ModuleStatus.evaluated;
        self.settleModulePromise(m, true, e);
        let parents = self.modules(m).asyncParents;
        for p in parents { self.asyncModuleRejected(p, e); }
    }

    /// Settles the promise of `m`'s cycle root, if it's waited on.
    mutating func settleModulePromise(m: Int64, rejected: Bool, e: Value) {
        if self.cycleRootOf(m) != m and self.modules(m).cycleRoot >= 0 { return; }
        let p = self.modules(m).promise;
        if not p.isObject { return; }
        if rejected { self.rejectPromise(p, e); } else { self.resolvePromise(p, Value.undefined); }
    }

    /// Load, link and evaluate `path`: a promise for its completion (a
    /// settled one for a synchronous graph).
    mutating func importModule(path: String) -> Result[(Int64, Value), Value] {
        let m = try self.loadModule(path);
        try self.linkModule(m);
        // An async graph reports through its root's promise.
        if not self.modules(m).promise.isObject {
            let p = self.newPromiseObject(self.intrinsics.promiseProto);
            self.modules(m).promise = p;
        }
        let p = self.modules(m).promise;
        self.pushRoot(p);
        match self.evaluateModule(m) {
            .Ok(_) => {
                let root = self.cycleRootOf(m);
                if root != m {
                    // Part of a cycle evaluated earlier: settle from the root.
                    let rr = self.modules(root);
                    if rr.hasError { self.rejectPromise(p, rr.error); } else if rr.status == ModuleStatus.evaluated { self.resolvePromise(p, Value.undefined); }
                } else if not self.modules(m).isAsyncEvaluation and self.modules(m).status == ModuleStatus.evaluated {
                    if self.modules(m).hasError { self.rejectPromise(p, self.modules(m).error); } else { self.resolvePromise(p, Value.undefined); }
                }
            },
            .Err(e) => self.rejectPromise(p, e)
        }
        .Ok((m, p))
    }

    /// Runs `path` as the entry module: loading or linking errors throw;
    /// evaluation errors reject the returned promise.
    mutating func runModuleFile(path: String) -> Result[Value, Value] {
        let (_, p) = try self.importModule(path);
        .Ok(p)
    }

    // ------------------------------------------------------------------
    // import() and import.meta
    // ------------------------------------------------------------------

    /// The module whose code frame `fi` runs, or -1 in a script.
    func frameModule(fi: Int64) -> Int64 {
        let callee = self.frames(fi).callee;
        if not callee.isObject { return -1; }
        match self.kindAt(callee.asRef) {
            .Closure(code, _, _, _) => self.codes(code).moduleId,
            _ => -1
        }
    }

    /// import(specifier, options): a promise for the module's namespace.
    /// The module loads on a later job, as it would from a real host.
    mutating func dynamicImport(fi: Int64, specifier: Value, options: Value) -> Value {
        let cap = self.newPromiseObject(self.intrinsics.promiseProto);
        self.pushRoot(cap);
        let spec = match self.toUnits(specifier) {
            .Ok(u) => textOfUnits(u),
            .Err(e) => {
                self.rejectPromise(cap, e);
                return cap;
            }
        };
        if let .Err(e) = self.checkImportOptions(options) {
            self.rejectPromise(cap, e);
            return cap;
        }
        let m = self.frameModule(fi);
        let referrer = if m >= 0 { self.modules(m).path } else { self.scriptPath };
        let path = self.str(resolveModulePath(referrer, spec));
        let start = self.nativeClosure(.ModuleImportStart, 0, [cap, path]);
        self.pushRoot(start);
        let ready = self.newPromiseObject(self.intrinsics.promiseProto);
        self.pushRoot(ready);
        self.resolvePromise(ready, Value.undefined);
        self.performThen(ready, start, Value.undefined, Value.undefined, Value.undefined, Value.undefined);
        cap
    }

    /// import()'s options: an object whose `with`, if present, is an
    /// object of string values.
    mutating func checkImportOptions(options: Value) -> Result[(), Value] {
        if options.isUndefined { return .Ok(()); }
        if not options.isObject { return self.throwType("The second argument of import() must be an object"); }
        let attrs = try self.getV(options, self.atom("with"));
        if attrs.isUndefined { return .Ok(()); }
        if not attrs.isObject { return self.throwType("The 'with' option of import() must be an object"); }
        for k in try self.ownKeys(attrs.asRef) {
            if keyIsSymbol(k) { continue; }
            let d = try self.getOwnPropertyV(attrs, k);
            let enumerable = match d { .Some(x) => x.enumerable ?? false, .None => false };
            if not enumerable { continue; }
            let v = try self.getV(attrs, k);
            if not v.isString { return self.throwType("Import attribute values must be strings"); }
        }
        .Ok(())
    }

    /// The import() job: load, link and evaluate, then settle `cap`.
    mutating func startImport(cap: Value, path: String) {
        match self.importModule(path) {
            .Ok((target, p)) => {
                let ns = self.moduleNamespace(target);
                self.pushRoot(ns);
                let onF = self.nativeClosure(.ModuleImportFulfilled, 1, [cap, ns]);
                self.pushRoot(onF);
                let onR = self.nativeClosure(.ModuleImportRejected, 1, [cap]);
                self.pushRoot(onR);
                self.performThen(p, onF, onR, Value.undefined, Value.undefined, Value.undefined);
            },
            .Err(e) => self.rejectPromise(cap, e)
        }
    }

    /// import.meta for frame `fi`'s module: a null-prototype object.
    mutating func importMeta(fi: Int64) -> Value {
        let m = self.frameModule(fi);
        if m < 0 { return Value.undefined; }
        if self.modules(m).meta.isObject { return self.modules(m).meta; }
        let o = self.newObjectKind(Value.nullValue, .Ordinary);
        self.modules(m).meta = o;
        o
    }

    mutating func moduleNative(n: NativeFn, callee: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        match n {
            .ModuleAsyncFulfilled => {
                let m = self.captures(callee)(0).asNumber.toInt64() ?? 0;
                self.asyncModuleFulfilled(m);
            },
            .ModuleAsyncRejected => {
                let m = self.captures(callee)(0).asNumber.toInt64() ?? 0;
                self.asyncModuleRejected(m, a0);
            },
            .ModuleImportStart => {
                let cs = self.captures(callee);
                let path = textOfUnits(self.units(cs(1)));
                self.startImport(cs(0), path);
            },
            .ModuleImportFulfilled => {
                let cs = self.captures(callee);
                self.resolvePromise(cs(0), cs(1));
            },
            .ModuleImportRejected => {
                let cs = self.captures(callee);
                self.rejectPromise(cs(0), a0);
            },
            _ => return .Ok(.None)
        }
        .Ok(.Some(Value.undefined))
    }
}

// ----------------------------------------------------------------------
// Entries
// ----------------------------------------------------------------------

/// The import and export entries of a module (ParseModule).
func collectModuleEntries(p: Program, mutating rec: ModuleRecord) {
    var imports: [ImportEntry] = [];
    var exports: [ExportEntry] = [];
    for s in p.body {
        match s.kind {
            .Import(d) => {
                addRequest(rec, d.source);
                for sp in d.specifiers {
                    match sp {
                        .Default(l) => imports.append(ImportEntry(request: d.source, importName: "default", localName: l)),
                        .Namespace(l) => imports.append(ImportEntry(request: d.source, importName: "*", localName: l)),
                        .Named(i, l) => imports.append(ImportEntry(request: d.source, importName: i, localName: l))
                    }
                }
            },
            .Export(e) => match e {
                .Declaration(d) => {
                    for n in exportDeclaredNames(d) { exports.append(ExportEntry(exportName: n, request: "", importName: "", localName: n)); }
                },
                .Default(d) => {
                    let local = match d.kind {
                        .FunctionDecl(f) => f.id ?? "*default*",
                        .ClassDecl(c) => c.id ?? "*default*",
                        _ => "*default*"
                    };
                    exports.append(ExportEntry(exportName: "default", request: "", importName: "", localName: local));
                },
                .Named(specs, source, _) => {
                    if let .Some(src) = source {
                        addRequest(rec, src);
                        for sp in specs { exports.append(ExportEntry(exportName: sp.exported, request: src, importName: sp.local, localName: "")); }
                    } else {
                        for sp in specs { exports.append(ExportEntry(exportName: sp.exported, request: "", importName: "", localName: sp.local)); }
                    }
                },
                .All(exported, source, _) => {
                    addRequest(rec, source);
                    match exported {
                        .Some(n) => exports.append(ExportEntry(exportName: n, request: source, importName: "*", localName: "")),
                        .None => exports.append(ExportEntry(exportName: "", request: source, importName: "*", localName: ""))
                    }
                }
            },
            _ => {}
        }
    }
    rec.imports = imports;
    for e in exports {
        if not e.request.isEmpty {
            if e.exportName.isEmpty { rec.starExports.append(e); } else { rec.indirectExports.append(e); }
            continue;
        }
        // A re-exported import is indirect, unless it's a namespace import.
        var found = false;
        for imp in imports {
            if imp.localName != e.localName { continue; }
            found = true;
            if imp.importName == "*" {
                rec.localExports.append(e);
            } else {
                rec.indirectExports.append(ExportEntry(exportName: e.exportName, request: imp.request, importName: imp.importName, localName: ""));
            }
        }
        if not found { rec.localExports.append(e); }
    }
}

func addRequest(mutating rec: ModuleRecord, spec: String) {
    if not rec.requests.contains(spec) { rec.requests.append(spec); }
}

/// The names a declaration statement binds.
func exportDeclaredNames(s: Stmt) -> Array[String] {
    var out: [String] = [];
    match s.kind {
        .Var(d) => { for decl in d.declarations { collectNames(decl.target, out); } },
        .FunctionDecl(f) => if let .Some(n) = f.id { out.append(n); },
        .ClassDecl(c) => if let .Some(n) = c.id { out.append(n); },
        _ => {}
    }
    out
}

/// A Kestrel string from code units (lone surrogates become U+FFFD).
func textOfUnits(u: Units) -> String {
    var s = "";
    var i: Int64 = 0;
    while i < u.count {
        var c = Int64(from: u(i));
        if c >= 0xD800 and c <= 0xDBFF and i + 1 < u.count {
            let d = Int64(from: u(i + 1));
            if d >= 0xDC00 and d <= 0xDFFF {
                c = 0x10000 + ((c - 0xD800) << 10) + (d - 0xDC00);
                i = i + 1;
            }
        }
        if c >= 0xD800 and c <= 0xDFFF { c = 0xFFFD; }
        s.append(char: Char(unchecked: UInt32(truncating: c)));
        i = i + 1;
    }
    s
}
