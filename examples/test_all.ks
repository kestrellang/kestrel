// Minimal self-contained implementations of String, Array[T], and UInt32
// No std2 imports - everything defined inline

module test

// === FFI ===

@extern(.C, mangleName: "malloc")
func malloc(consuming size: lang.i64) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "free")
func free(consuming ptr: lang.ptr[lang.i8])

@extern(.C, mangleName: "exit")
func libc_exit(code: lang.i32)

// === Protocols ===

protocol Equatable {
    func equals(other: Self) -> Bool
}

protocol ExpressibleByIntLiteral {
    init(intLiteral value: lang.i64)
}

protocol ExpressibleByBoolLiteral {
    init(boolLiteral value: lang.i1)
}

protocol BooleanConditional {
    func boolValue() -> lang.i1
}

protocol Addable {
    type Output
    func add(other: Self) -> Output
}

protocol Subtractable {
    type Output
    func subtract(other: Self) -> Output
}

protocol Multipliable {
    type Output
    func multiply(other: Self) -> Output
}

// === Bool ===

struct Bool: Equatable, ExpressibleByBoolLiteral, BooleanConditional {
    var value: lang.i1

    init(boolLiteral value: lang.i1) {
        self.value = value
    }

    func equals(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    func boolValue() -> lang.i1 {
        self.value
    }
}

// === Int64 ===

struct Int64: Equatable, ExpressibleByIntLiteral, Addable, Subtractable, Multipliable {
    var raw: lang.i64

    type Addable.Output = Int64
    type Subtractable.Output = Int64
    type Multipliable.Output = Int64

    init(intLiteral value: lang.i64) {
        self.raw = value
    }

    init(raw value: lang.i64) {
        self.raw = value
    }

    func equals(other: Int64) -> Bool {
        Bool(boolLiteral: lang.i64_eq(self.raw, other.raw))
    }

    func add(other: Int64) -> Int64 {
        Int64(raw: lang.i64_add(self.raw, other.raw))
    }

    func subtract(other: Int64) -> Int64 {
        Int64(raw: lang.i64_sub(self.raw, other.raw))
    }

    func multiply(other: Int64) -> Int64 {
        Int64(raw: lang.i64_mul(self.raw, other.raw))
    }
}

// === UInt8 ===

struct UInt8: Equatable, ExpressibleByIntLiteral, Addable {
    var raw: lang.i8

    type Addable.Output = UInt8

    init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i8(value)
    }

    init(raw value: lang.i8) {
        self.raw = value
    }

    func equals(other: UInt8) -> Bool {
        Bool(boolLiteral: lang.i8_eq(self.raw, other.raw))
    }

    func add(other: UInt8) -> UInt8 {
        UInt8(raw: lang.i8_add(self.raw, other.raw))
    }
}

// === UInt32 ===

struct UInt32: Equatable, ExpressibleByIntLiteral, Addable, Subtractable, Multipliable {
    var raw: lang.i32

    type Addable.Output = UInt32
    type Subtractable.Output = UInt32
    type Multipliable.Output = UInt32

    init(intLiteral value: lang.i64) {
        self.raw = lang.cast_i64_i32(value)
    }

    init(raw value: lang.i32) {
        self.raw = value
    }

    func equals(other: UInt32) -> Bool {
        Bool(boolLiteral: lang.i32_eq(self.raw, other.raw))
    }

    func add(other: UInt32) -> UInt32 {
        UInt32(raw: lang.i32_add(self.raw, other.raw))
    }

    func subtract(other: UInt32) -> UInt32 {
        UInt32(raw: lang.i32_sub(self.raw, other.raw))
    }

    func multiply(other: UInt32) -> UInt32 {
        UInt32(raw: lang.i32_mul(self.raw, other.raw))
    }
}

// === RawPointer ===

struct RawPointer {
    var raw: lang.ptr[lang.i8]

    init(raw: lang.ptr[lang.i8]) {
        self.raw = raw
    }

    func cast[T]() -> Pointer[T] {
        Pointer(raw: lang.cast_ptr[T](self.raw))
    }
}

// === Pointer[T] ===

struct Pointer[T] {
    var raw: lang.ptr[T]

    init(raw: lang.ptr[T]) {
        self.raw = raw
    }

    func read() -> T {
        lang.ptr_read(self.raw)
    }

    func write(value: T) {
        lang.ptr_write(self.raw, value)
    }

    func offset(by: Int64) -> Pointer[T] {
        Pointer(raw: lang.ptr_offset(self.raw, by.raw))
    }

    func asRaw() -> RawPointer {
        RawPointer(raw: lang.cast_ptr[lang.i8](self.raw))
    }
}

// === Array[T] ===

struct Array[T] {
    var ptr: Pointer[T]
    var len: Int64
    var cap: Int64

    init() {
        self.ptr = Pointer(raw: lang.ptr_null[T]());
        self.len = Int64(intLiteral: 0);
        self.cap = Int64(intLiteral: 0);
    }

    init(capacity: Int64) {
        if capacity.raw > 0 {
            let size = lang.i64_mul(capacity.raw, lang.size_of[T]());
            let rawPtr = malloc(size);
            self.ptr = RawPointer(raw: rawPtr).cast[T]();
            self.len = Int64(intLiteral: 0);
            self.cap = capacity
        } else {
            self.ptr = Pointer(raw: lang.ptr_null[T]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    deinit {
        if self.cap.raw > 0 {
            free(self.ptr.asRaw().raw)
        }
    }

    func count() -> Int64 {
        self.len
    }

    mutating func append(element: T) {
        // Simple growth: if full, allocate new buffer with 2x capacity
        if self.len.raw >= self.cap.raw {
            var newCap: lang.i64 = self.cap.raw;
            if newCap == 0 {
                newCap = 4
            } else {
                newCap = lang.i64_mul(newCap, 2)
            }
            let size = lang.i64_mul(newCap, lang.size_of[T]());
            let newRaw = malloc(size);
            let newPtr: Pointer[T] = RawPointer(raw: newRaw).cast[T]();

            // Copy existing elements
            var i: lang.i64 = 0;
            while lang.i64_signed_lt(i, self.len.raw) {
                newPtr.offset(Int64(raw: i)).write(self.ptr.offset(Int64(raw: i)).read());
                i = lang.i64_add(i, 1)
            }

            // Free old buffer
            if self.cap.raw > 0 {
                free(self.ptr.asRaw().raw)
            }

            self.ptr = newPtr;
            self.cap = Int64(raw: newCap)
        }

        self.ptr.offset(self.len).write(element);
        self.len = self.len.add(Int64(intLiteral: 1))
    }

    func getUnchecked(index: Int64) -> T {
        self.ptr.offset(index).read()
    }
}

// === String ===

struct String {
    var ptr: Pointer[UInt8]
    var len: Int64
    var cap: Int64

    init() {
        self.ptr = Pointer(raw: lang.ptr_null[UInt8]());
        self.len = Int64(intLiteral: 0);
        self.cap = Int64(intLiteral: 0);
    }

    init(capacity: Int64) {
        if capacity.raw > 0 {
            let rawPtr = malloc(capacity.raw);
            self.ptr = RawPointer(raw: rawPtr).cast[UInt8]();
            self.len = Int64(intLiteral: 0);
            self.cap = capacity
        } else {
            self.ptr = Pointer(raw: lang.ptr_null[UInt8]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    deinit {
        if self.cap.raw > 0 {
            free(self.ptr.asRaw().raw)
        }
    }

    func byteCount() -> Int64 {
        self.len
    }

    mutating func appendByte(byte: UInt8) {
        // Simple growth: if full, allocate new buffer with 2x capacity
        if self.len.raw >= self.cap.raw {
            var newCap: lang.i64 = self.cap.raw;
            if newCap == 0 {
                newCap = 16
            } else {
                newCap = lang.i64_mul(newCap, 2)
            }
            let newRaw = malloc(newCap);
            let newPtr: Pointer[UInt8] = RawPointer(raw: newRaw).cast[UInt8]();

            // Copy existing bytes
            var i: lang.i64 = 0;
            while lang.i64_signed_lt(i, self.len.raw) {
                newPtr.offset(Int64(raw: i)).write(self.ptr.offset(Int64(raw: i)).read());
                i = lang.i64_add(i, 1)
            }

            // Free old buffer
            if self.cap.raw > 0 {
                free(self.ptr.asRaw().raw)
            }

            self.ptr = newPtr;
            self.cap = Int64(raw: newCap)
        }

        self.ptr.offset(self.len).write(byte);
        self.len = self.len.add(Int64(intLiteral: 1))
    }

    func byteAtUnchecked(index: Int64) -> UInt8 {
        self.ptr.offset(index).read()
    }
}

// === Test Code ===

func main() {
    // Test UInt32 arithmetic
    let a = UInt32(intLiteral: 10);
    let b = UInt32(intLiteral: 20);
    let sum = a.add(b);
    let expectedSum = UInt32(intLiteral: 30);

    // Test Array[UInt8]
    var arr = Array[UInt8](capacity: Int64(intLiteral: 4));
    arr.append(UInt8(intLiteral: 65));  // 'A'
    arr.append(UInt8(intLiteral: 66));  // 'B'
    let first = arr.getUnchecked(Int64(intLiteral: 0));
    let expectedFirst = UInt8(intLiteral: 65);

    // Test String
    var str = String(capacity: Int64(intLiteral: 8));
    str.appendByte(UInt8(intLiteral: 72));  // 'H'
    str.appendByte(UInt8(intLiteral: 105)); // 'i'
    let h = str.byteAtUnchecked(Int64(intLiteral: 0));
    let expectedH = UInt8(intLiteral: 72);

    // Verify results
    let sumOk = sum.equals(expectedSum);
    let arrOk = first.equals(expectedFirst);
    let strOk = h.equals(expectedH);

    if sumOk.boolValue() and arrOk.boolValue() and strOk.boolValue() {
        libc_exit(0)  // Success
    } else {
        libc_exit(1)  // Failure
    }
}
