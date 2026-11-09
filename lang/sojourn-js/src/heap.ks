module js

// The garbage-collected heap.
//
// Cells live in typed spaces (objects, strings as UTF-16 code units,
// environments), each an
// array addressed by index with its own free list and mark bits. A
// `Value` names a cell by tag and index. Collection is mark-sweep and
// never moves cells, so an index stays valid for the cell's lifetime.
//
// Collection only happens at the interpreter's safe points, where every
// live value is in a register, a frame, the VM's tables or the handle
// stack. Native code that keeps a value across a call back into JS roots
// it with `pushRoot`.

import std.collections.(Dictionary)

/// A property key: an atom (an interned string) when >= 0, or a symbol
/// when < 0 (symbol `n` is key `-n - 1`).
type Key = Int64;

func symbolKey(id: Int64) -> Key { 0 - id - 1 }
func keyIsSymbol(k: Key) -> Bool { k < 0 }
func symbolOfKey(k: Key) -> Int64 { 0 - k - 1 }

/// Property attributes, as bits.
struct Attr {
    static var writable: Int64 { 1 }
    static var enumerable: Int64 { 2 }
    static var configurable: Int64 { 4 }
    /// The slot holds an accessor object (`ObjectKind.AccessorPair`).
    static var accessor: Int64 { 8 }
    static var all: Int64 { 7 }
}

/// An object's layout: which keys it has, in order, and where each lives.
/// Objects built the same way share one, so a property access can be
/// cached as "this shape, this slot".
struct Shape {
    var keys: [Key]
    var attrs: [Int64]
    var slotOf: [Key: Int64]
    /// Shapes made by adding one key with all attributes set.
    var transitions: [Key: Int64]
    /// Shared shapes are immutable; an object that deletes keys or grows
    /// large gets a shape of its own ("dictionary mode") and edits it.
    var shared: Bool
}

/// What kind of object, with the internal slots that go with it.
enum ObjectKind {
    case Ordinary
    case ArrayObject
    /// A function compiled from JS: its code, the environment it closes
    /// over, its home object (for `super`), and for class constructors
    /// the field initializer.
    case Closure(code: Int64, env: Int64, home: Value, fields: Value)
    case NativeFunction(NativeFn)
    case BoundFunction(target: Value, boundThis: Value, args: [Value])
    case ErrorObject
    case BooleanBox(Bool)
    case NumberBox(Float64)
    case StringBox(Value)
    case SymbolBox(Value)
    case ArgumentsObject
    /// A getter/setter pair stored in an accessor property's slot.
    case AccessorPair(getter: Value, setter: Value)
    /// For-in state: the keys left to visit.
    case KeyIterator(object: Value, keys: [Value], index: Int64)
    /// Iterators over arrays (and array-likes) and strings.
    case ArrayIterator(target: Value, index: Int64, kind: Int64)
    case StringIterator(target: Value, index: Int64)
    /// A Map, Set, WeakMap or WeakSet (`CollectionKind`): its entries
    /// are `Heap.maps(id)`.
    case Collection(id: Int64, kind: Int64)
    /// `kind` 0 yields keys, 1 values, 2 [key, value] pairs.
    case CollectionIterator(target: Value, index: Int64, kind: Int64)
    /// A Date: its time value (NaN for an invalid date).
    case DateObject(Float64)
    /// A generator object: its suspended frame is `Heap.generators(id)`.
    case Generator(id: Int64)
}

/// Generator states.
struct GenState {
    static var suspendedStart: Int64 { 0 }
    static var suspendedYield: Int64 { 1 }
    static var executing: Int64 { 2 }
    static var completed: Int64 { 3 }
}

/// A generator's suspended frame and its registers.
struct GenData {
    var frame: Frame
    var registers: [Value]
    var state: Int64
    /// Where the next resume writes the sent value (and mode after it).
    var resumeReg: Int64
    /// Whether the last yield handed out an iterator result as is.
    var raw: Bool
}

/// A collection's entries in insertion order. Deleted entries stay as
/// `Value.empty` holes so iterators keep their positions.
struct MapData {
    var keys: [Value]
    var values: [Value]
    /// `VM.collectionKey` of each live key to its position.
    var index: [UInt64: Int64]
    var size: Int64
}

struct JSObject {
    var shape: Int64
    var slots: [Value]
    /// The prototype: an object, or null.
    var proto: Value
    /// Dense indexed elements, for arrays. Holes are `Value.empty`.
    /// Elements with attributes, accessors, and ones far past the dense
    /// part live in the shape under their index keys instead.
    var elements: [Value]
    /// Arrays only: `length`, which may exceed `elements.count`.
    var length: Int64
    /// Arrays only: whether any index lives in the shape, which turns
    /// off the natives' dense fast paths.
    var hasShapeIndices: Bool
    var kind: ObjectKind
    var extensible: Bool
    /// Functions only: whether `new` works on it.
    var isConstructor: Bool
}

/// A scope's variables at run time.
struct Env {
    var slots: [Value]
    /// The enclosing environment, or -1.
    var parent: Int64
}

struct Heap {
    var objects: [JSObject]
    var objectMarks: [Bool]
    var freeObjects: [Int64]
    var strings: [[UInt16]]
    var stringMarks: [Bool]
    var stringLive: [Bool]
    var freeStrings: [Int64]
    var envs: [Env]
    var envMarks: [Bool]
    var freeEnvs: [Int64]
    /// Collection entries, owned by one `ObjectKind.Collection` object
    /// each and freed with it.
    var maps: [MapData]
    var freeMaps: [Int64]
    var generators: [GenData]
    var freeGenerators: [Int64]
    /// Cells allocated since the last collection, which decides when to
    /// collect next.
    var allocated: Int64
    var threshold: Int64

    init() {
        self.objects = [];
        self.objectMarks = [];
        self.freeObjects = [];
        self.strings = [];
        self.stringMarks = [];
        self.stringLive = [];
        self.freeStrings = [];
        self.envs = [];
        self.envMarks = [];
        self.freeEnvs = [];
        self.maps = [];
        self.freeMaps = [];
        self.generators = [];
        self.freeGenerators = [];
        self.allocated = 0;
        self.threshold = 100000;
    }

    mutating func allocObject(o: JSObject) -> Ref {
        self.allocated = self.allocated + 1;
        if let .Some(r) = self.freeObjects.pop() {
            self.objects(r) = o;
            return r;
        }
        self.objects.append(o);
        self.objectMarks.append(false);
        self.objects.count - 1
    }

    mutating func allocString(s: Array[UInt16]) -> Ref {
        self.allocated = self.allocated + 1;
        if let .Some(r) = self.freeStrings.pop() {
            self.strings(r) = s;
            self.stringLive(r) = true;
            return r;
        }
        self.strings.append(s);
        self.stringMarks.append(false);
        self.stringLive.append(true);
        self.strings.count - 1
    }

    mutating func allocEnv(e: Env) -> Ref {
        self.allocated = self.allocated + 1;
        if let .Some(r) = self.freeEnvs.pop() {
            self.envs(r) = e;
            return r;
        }
        self.envs.append(e);
        self.envMarks.append(false);
        self.envs.count - 1
    }

    mutating func allocMap() -> Int64 {
        let empty = MapData(keys: [], values: [], index: [:], size: 0);
        if let .Some(id) = self.freeMaps.pop() {
            self.maps(id) = empty;
            return id;
        }
        self.maps.append(empty);
        self.maps.count - 1
    }

    mutating func allocGenerator(g: GenData) -> Int64 {
        if let .Some(id) = self.freeGenerators.pop() {
            self.generators(id) = g;
            return id;
        }
        self.generators.append(g);
        self.generators.count - 1
    }

    func wantsCollection() -> Bool { self.allocated >= self.threshold }

    // ------------------------------------------------------------------
    // Mark and sweep
    // ------------------------------------------------------------------

    /// Marks everything reachable from `roots` and `envRoots`, then frees
    /// the rest.
    mutating func collect(roots: Array[Value], envRoots: Array[Int64]) {
        var marker = Marker(values: [], envs: []);
        for v in roots { marker.values.append(v); }
        for e in envRoots { marker.envs.append(e); }
        self.drain(marker);
        self.sweep();
        self.allocated = 0;
        // Collect again once the heap has grown by about as much as survived.
        let live = self.objects.count - self.freeObjects.count + self.strings.count - self.freeStrings.count;
        self.threshold = if live < 100000 { 100000 } else { live };
    }

    mutating func drain(mutating m: Marker) {
        while true {
            if let .Some(v) = m.values.pop() {
                self.markValue(v, m);
                continue;
            }
            if let .Some(e) = m.envs.pop() {
                if e < 0 or self.envMarks(e) { continue; }
                self.envMarks(e) = true;
                let env = self.envs(e);
                for s in env.slots { m.values.append(s); }
                m.envs.append(env.parent);
                continue;
            }
            break;
        }
    }

    mutating func markValue(v: Value, mutating m: Marker) {
        if v.isString {
            self.stringMarks(v.asRef) = true;
            return;
        }
        if not v.isObject { return; }
        let r = v.asRef;
        if self.objectMarks(r) == true { return; }
        self.objectMarks(r) = true;
        let o = self.objects(r);
        for s in o.slots { m.values.append(s); }
        for s in o.elements { m.values.append(s); }
        m.values.append(o.proto);
        if let .Collection(id, _) = o.kind {
            for k in self.maps(id).keys { m.values.append(k); }
            for x in self.maps(id).values { m.values.append(x); }
        }
        if let .Generator(id) = o.kind {
            let g = self.generators(id);
            for x in g.registers { m.values.append(x); }
            let f = g.frame;
            m.envs.append(f.env);
            m.envs.append(f.funcEnv);
            for a in f.args { m.values.append(a); }
            m.values.append(f.callee);
            m.values.append(f.newTarget);
            for h in f.handlers { m.envs.append(h.env); }
        }
        traceKind(o.kind, m);
    }

    mutating func sweep() {
        for i in 0..<self.objects.count {
            if self.objectMarks(i) == true {
                self.objectMarks(i) = false;
            } else if not isFreeObject(self.objects(i)) {
                let kind = self.objects(i).kind;
                if let .Collection(id, _) = kind {
                    self.maps(id) = MapData(keys: [], values: [], index: [:], size: 0);
                    self.freeMaps.append(id);
                }
                if let .Generator(id) = kind {
                    self.generators(id).registers = [];
                    self.generators(id).frame.args = [];
                    self.generators(id).state = GenState.completed;
                    self.freeGenerators.append(id);
                }
                self.objects(i) = freeObject();
                self.freeObjects.append(i);
            }
        }
        for i in 0..<self.strings.count {
            if self.stringMarks(i) == true {
                self.stringMarks(i) = false;
            } else if self.stringLive(i) == true {
                self.strings(i) = [];
                self.stringLive(i) = false;
                self.freeStrings.append(i);
            }
        }
        for i in 0..<self.envs.count {
            if self.envMarks(i) == true {
                self.envMarks(i) = false;
            } else if self.envs(i).parent != -2 {
                // parent -2 marks a free env.
                self.envs(i) = Env(slots: [], parent: -2);
                self.freeEnvs.append(i);
            }
        }
    }
}

/// The GC's worklist.
struct Marker {
    var values: [Value]
    var envs: [Int64]
}

/// Pushes the cells an object's internal slots refer to.
func traceKind(kind: ObjectKind, mutating m: Marker) {
    match kind {
        .Closure(_, env, home, fields) => {
            m.envs.append(env);
            m.values.append(home);
            m.values.append(fields);
        },
        .BoundFunction(target, boundThis, args) => {
            m.values.append(target);
            m.values.append(boundThis);
            for a in args { m.values.append(a); }
        },
        .StringBox(s) => m.values.append(s),
        .SymbolBox(s) => m.values.append(s),
        .AccessorPair(g, s) => {
            m.values.append(g);
            m.values.append(s);
        },
        .KeyIterator(o, keys, _) => {
            m.values.append(o);
            for k in keys { m.values.append(k); }
        },
        .ArrayIterator(t, _, _) => m.values.append(t),
        .StringIterator(t, _) => m.values.append(t),
        .CollectionIterator(t, _, _) => m.values.append(t),
        _ => {}
    }
}

/// A freed object slot: shape -1.
func freeObject() -> JSObject {
    JSObject(shape: -1, slots: [], proto: Value.nullValue, elements: [], length: 0, hasShapeIndices: false, kind: .Ordinary, extensible: false, isConstructor: false)
}

func isFreeObject(o: JSObject) -> Bool { o.shape == -1 }
