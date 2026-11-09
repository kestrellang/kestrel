module js

// The virtual machine: heap, compiled code, the call stack, and the
// realm's built-in objects.

import std.collections.(Dictionary, Set)

struct InlineCache {
    var shape: Int64
    var slot: Int64
}

/// A try block's handler, active while the block runs.
struct Handler {
    var pc: Int64
    var exception: Int64
    var env: Int64
}

struct Frame {
    var code: Int64
    var pc: Int64
    /// Where the frame's registers start in `stack`.
    var base: Int64
    var env: Int64
    /// The function's own environment, holding `this` in slot 0.
    var funcEnv: Int64
    var args: [Value]
    var callee: Value
    var newTarget: Value
    var isConstruct: Bool
    /// The caller's register (absolute) that gets the result, or -1.
    var returnTo: Int64
    var handlers: [Handler]
    /// The generator object a generator body's frame belongs to.
    var generator: Value
}

/// The realm's prototypes and well-known objects.
struct Intrinsics {
    var objectProto: Value
    var functionProto: Value
    var arrayProto: Value
    var stringProto: Value
    var numberProto: Value
    var booleanProto: Value
    var symbolProto: Value
    var errorProto: Value
    var typeErrorProto: Value
    var rangeErrorProto: Value
    var referenceErrorProto: Value
    var syntaxErrorProto: Value
    var evalErrorProto: Value
    var uriErrorProto: Value
    var iteratorProto: Value
    var arrayIteratorProto: Value
    var stringIteratorProto: Value
    var regexpProto: Value
    var mapIteratorProto: Value
    var setIteratorProto: Value
    var generatorProto: Value
    var generatorFunctionProto: Value
    var regexpStringIteratorProto: Value
    var throwTypeError: Value
    var promiseCtor: Value
    var promiseProto: Value
    var asyncFunctionProto: Value
    var aggregateErrorProto: Value
    var asyncIteratorProto: Value
    var asyncFromSyncIteratorProto: Value
    var asyncGeneratorFunctionProto: Value
    var asyncGeneratorProto: Value
    var iteratorHelperProto: Value
    var wrapForValidIteratorProto: Value
    var arrayBufferProto: Value
    var typedArrayProto: Value
    var typedArrayCtor: Value
    var dataViewProto: Value
}

/// Well-known symbols, by symbol id.
struct WellKnown {
    static var iterator: Int64 { 0 }
    static var hasInstance: Int64 { 1 }
    static var toPrimitive: Int64 { 2 }
    static var toStringTag: Int64 { 3 }
    static var unscopables: Int64 { 4 }
    static var asyncIterator: Int64 { 5 }
    static var isConcatSpreadable: Int64 { 6 }
    static var species: Int64 { 7 }
    static var matchSym: Int64 { 8 }
    static var matchAll: Int64 { 9 }
    static var replace: Int64 { 10 }
    static var search: Int64 { 11 }
    static var split: Int64 { 12 }
}

/// Atoms the engine uses often.
struct Names {
    var length: Key
    var prototype: Key
    var constructor: Key
    var name: Key
    var message: Key
    var toString: Key
    var valueOf: Key
    var next: Key
    var done: Key
    var value: Key
    var getKey: Key
    var setKey: Key
    var writable: Key
    var enumerable: Key
    var configurable: Key
    var callee: Key
    var returnKey: Key
    var raw: Key
    var cause: Key
    var lastIndex: Key
}

struct VM {
    var heap: Heap
    var names: Names
    var shapes: [Shape]
    var atomIds: [String: Int64]
    var atomNames: [String]
    /// Each atom's string value, kept alive for the program's life.
    var atomValues: [Value]
    /// Each atom's array index, or -1.
    var atomIndex: [Int64]
    /// Symbol descriptions (a string value or undefined), by id.
    var symbols: [Value]
    /// Symbol ids that are class private names.
    var privateSymbols: Set[Int64]
    var symbolRegistry: [String: Int64]
    var codes: [Code]
    var caches: [InlineCache]
    var fns: [FnState]
    /// Script-level lexical names seen so far (compile time).
    var globalLexicalNames: Set[String]
    /// Slots of script-level lexical bindings in `globalEnv`.
    var globalLexicals: [Int64: Int64]
    var globalConsts: Set[Int64]
    var globalEnv: Int64
    var stack: [Value]
    var frames: [Frame]
    /// Values native code keeps alive across calls into JS.
    var roots: [Value]
    var global: Value
    var intrinsics: Intrinsics
    var sourceName: String
    /// Calls deep, to turn runaway recursion into a RangeError.
    var depth: Int64
    var randomState: UInt64
    /// Compiled regular expressions, and their ids by "flags/source".
    var regexPrograms: [RxProgram]
    var regexCache: [String: Int64]
    /// Promise jobs waiting to run, oldest first (`runJobs`).
    var jobs: [Job]
    /// The concrete typed array prototypes and constructors, by TAKind.
    var typedArrayProtos: [Value]
    var typedArrayCtors: [Value]

    init() {
        self.heap = Heap();
        self.names = Names(length: 0, prototype: 0, constructor: 0, name: 0, message: 0, toString: 0,
            valueOf: 0, next: 0, done: 0, value: 0, getKey: 0, setKey: 0, writable: 0, enumerable: 0,
            configurable: 0, callee: 0, returnKey: 0, raw: 0, cause: 0, lastIndex: 0);
        self.shapes = [Shape(keys: [], attrs: [], slotOf: [:], transitions: [:], shared: true)];
        self.atomIds = [:];
        self.atomNames = [];
        self.atomValues = [];
        self.atomIndex = [];
        self.symbols = [];
        self.privateSymbols = Set();
        self.symbolRegistry = [:];
        self.codes = [];
        self.caches = [];
        self.fns = [];
        self.globalLexicalNames = Set();
        self.globalLexicals = [:];
        self.globalConsts = Set();
        self.globalEnv = 0;
        self.stack = [];
        self.frames = [];
        self.roots = [];
        self.global = Value.undefined;
        let u = Value.undefined;
        self.intrinsics = Intrinsics(
            objectProto: u, functionProto: u, arrayProto: u, stringProto: u, numberProto: u,
            booleanProto: u, symbolProto: u, errorProto: u, typeErrorProto: u, rangeErrorProto: u,
            referenceErrorProto: u, syntaxErrorProto: u, evalErrorProto: u, uriErrorProto: u,
            iteratorProto: u, arrayIteratorProto: u, stringIteratorProto: u, regexpProto: u,
            mapIteratorProto: u, setIteratorProto: u, generatorProto: u, generatorFunctionProto: u,
            regexpStringIteratorProto: u,
            throwTypeError: u, promiseCtor: u, promiseProto: u, asyncFunctionProto: u, aggregateErrorProto: u,
            asyncIteratorProto: u, asyncFromSyncIteratorProto: u, asyncGeneratorFunctionProto: u, asyncGeneratorProto: u,
            iteratorHelperProto: u, wrapForValidIteratorProto: u,
            arrayBufferProto: u, typedArrayProto: u, typedArrayCtor: u, dataViewProto: u
        );
        self.sourceName = "";
        self.depth = 0;
        self.randomState = 0x2545F4914F6CDD1D;
        self.regexPrograms = [];
        self.regexCache = [:];
        self.jobs = [];
        self.typedArrayProtos = [];
        self.typedArrayCtors = [];
        self.globalEnv = self.heap.allocEnv(Env(slots: [], parent: -1));
        self.names = Names(
            length: self.atom("length"), prototype: self.atom("prototype"), constructor: self.atom("constructor"),
            name: self.atom("name"), message: self.atom("message"), toString: self.atom("toString"),
            valueOf: self.atom("valueOf"), next: self.atom("next"), done: self.atom("done"),
            value: self.atom("value"), getKey: self.atom("get"), setKey: self.atom("set"),
            writable: self.atom("writable"), enumerable: self.atom("enumerable"),
            configurable: self.atom("configurable"), callee: self.atom("callee"),
            returnKey: self.atom("return"), raw: self.atom("raw"), cause: self.atom("cause"),
            lastIndex: self.atom("lastIndex")
        );
        self.setupRealm();
    }

    // ------------------------------------------------------------------
    // Atoms, strings and symbols
    // ------------------------------------------------------------------

    /// The atom for `s`, interning it on first use.
    mutating func atom(s: String) -> Key {
        if let .Some(a) = self.atomIds(s) { return a; }
        let a = self.atomNames.count;
        let _ = self.atomIds.insert(s, a);
        self.atomNames.append(s);
        let r = self.heap.allocString(unitsOf(s));
        self.atomValues.append(Value.string(r));
        self.atomIndex.append(arrayIndexOf(s));
        a
    }

    mutating func atomOfUnits(u: Units) -> Key { self.atom(stringOf(u)) }

    mutating func newString(u: Units) -> Value { Value.string(self.heap.allocString(u)) }

    mutating func str(s: String) -> Value { self.newString(unitsOf(s)) }

    func units(v: Value) -> Units { self.heap.strings(v.asRef) }

    /// A string value's text, as a Kestrel string.
    func text(v: Value) -> String { stringOf(self.heap.strings(v.asRef)) }

    mutating func newSymbol(description: Value) -> Value {
        self.symbols.append(description);
        Value.symbol(self.symbols.count - 1)
    }

    /// A key's printable name, for error messages.
    func keyText(k: Key) -> String {
        if keyIsSymbol(k) {
            let d = self.symbols(symbolOfKey(k));
            if self.privateSymbols.contains(symbolOfKey(k)) { return self.text(d); }
            return if d.isString { "Symbol(\(self.text(d)))" } else { "Symbol()" };
        }
        self.atomNames(k)
    }

    // ------------------------------------------------------------------
    // Errors
    // ------------------------------------------------------------------

    /// A new error object of `kind` with `message`, to throw.
    mutating func makeError(kind: ErrorKind, message: String) -> Value {
        let proto = match kind {
            .TypeError => self.intrinsics.typeErrorProto,
            .ReferenceError => self.intrinsics.referenceErrorProto,
            .RangeError => self.intrinsics.rangeErrorProto,
            .SyntaxError => self.intrinsics.syntaxErrorProto,
            .URIError => self.intrinsics.uriErrorProto
        };
        let e = self.newObjectKind(proto, .ErrorObject);
        let m = self.str(message);
        let key = self.atom("message");
        self.defineOwn(e.asRef, key, m, Attr.writable | Attr.configurable);
        e
    }

    mutating func typeError(message: String) -> Value { self.makeError(.TypeError, message) }

    /// `Err` of a new TypeError, for `try`ing callers.
    mutating func throwType[T](message: String) -> Result[T, Value] { .Err(self.makeError(.TypeError, message)) }

    // ------------------------------------------------------------------
    // Garbage collection
    // ------------------------------------------------------------------

    /// Collects garbage, with everything the VM and its frames hold as roots.
    mutating func collectGarbage() {
        var values: [Value] = [];
        var envs: [Int64] = [self.globalEnv];
        for v in self.stack { values.append(v); }
        for v in self.roots { values.append(v); }
        for v in self.atomValues { values.append(v); }
        for v in self.symbols { values.append(v); }
        values.append(self.global);
        for v in self.intrinsicValues() { values.append(v); }
        for f in self.frames {
            envs.append(f.env);
            envs.append(f.funcEnv);
            for a in f.args { values.append(a); }
            values.append(f.callee);
            values.append(f.newTarget);
            for h in f.handlers { envs.append(h.env); }
            values.append(f.generator);
        }
        for j in self.jobs {
            values.append(j.a);
            values.append(j.b);
            values.append(j.c);
            values.append(j.d);
            values.append(j.e);
        }
        self.heap.collect(values, envs);
    }

    /// Every intrinsic, for the GC. A new `Intrinsics` field goes here too.
    func intrinsicValues() -> Array[Value] {
        let i = self.intrinsics;
        var out: Array[Value] = [i.objectProto, i.functionProto, i.arrayProto, i.stringProto, i.numberProto, i.booleanProto,
         i.symbolProto, i.errorProto, i.typeErrorProto, i.rangeErrorProto, i.referenceErrorProto,
         i.syntaxErrorProto, i.evalErrorProto, i.uriErrorProto, i.iteratorProto,
         i.arrayIteratorProto, i.stringIteratorProto, i.regexpProto, i.mapIteratorProto,
         i.setIteratorProto, i.generatorProto, i.generatorFunctionProto, i.regexpStringIteratorProto,
         i.throwTypeError, i.promiseCtor, i.promiseProto, i.asyncFunctionProto, i.aggregateErrorProto,
         i.asyncIteratorProto, i.asyncFromSyncIteratorProto, i.asyncGeneratorFunctionProto, i.asyncGeneratorProto,
         i.iteratorHelperProto, i.wrapForValidIteratorProto,
         i.arrayBufferProto, i.typedArrayProto, i.typedArrayCtor, i.dataViewProto];
        for v in self.typedArrayProtos { out.append(v); }
        for v in self.typedArrayCtors { out.append(v); }
        out
    }

    mutating func pushRoot(v: Value) { self.roots.append(v); }

    mutating func popRoots(n: Int64) {
        for _ in 0..<n { let _ = self.roots.pop(); }
    }

    /// Drops roots back to an earlier `roots.count`.
    mutating func truncateRoots(mark: Int64) {
        while self.roots.count > mark { let _ = self.roots.pop(); }
    }
}
