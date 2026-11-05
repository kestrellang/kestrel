// test: execution
// stdlib: true
// backends: cranelift,llvm
// c-source: c_abi_callbacks.c
// expect-stdout: 42\n105\n43\n-1\n1\n0\n321\ntrue\n3006\n5\n

// Naming an `@export(.C)` function where a C function type
// (`@extern(.C) (…) -> …`) is expected hands it to C as a callback: as a
// direct argument, inside an `FFISafe` struct (with the closure state in the
// C API's `userdata`), and with struct arguments and results in both
// directions. `NullableCFunction` carries a callback that may be NULL.

module Test

struct I32x2: FFISafe { var a: Int32; var b: Int32 }
struct F32x3: FFISafe { var x: Float32; var y: Float32; var z: Float32 }

type IntFn = @extern(.C) (Int64, Pointer[Int64]) -> Int64

// wgpu's `*CallbackInfo` shape: 40 bytes, so C passes it on the stack.
struct ModeCallbackInfo: FFISafe {
    var next: RawPointer;
    var mode: UInt32;
    var callback: IntFn;
    var userdata1: Pointer[Int64];
    var userdata2: RawPointer;
}

// wgpu's `WGPUStringView`: C passes it to callbacks in two registers.
struct View: FFISafe { var data: Pointer[UInt8]; var length: UInt64 }

struct CallbackInfo: FFISafe {
    var callback: NullableCFunction[IntFn];
    var userdata: Pointer[Int64];
}

@export(.C, mangleName: "kestrel_add_userdata")
func addUserdata(x: Int64, userdata: Pointer[Int64]) -> Int64 {
    x + unsafe userdata.read()
}

@export(.C, mangleName: "kestrel_swap_pair")
func swapPair(v: I32x2) -> I32x2 { I32x2(a: v.b, b: v.a) }

@export(.C, mangleName: "kestrel_reverse_vec")
func reverseVec(v: F32x3) -> F32x3 { F32x3(x: v.z, y: v.y, z: v.x) }

@export(.C, mangleName: "kestrel_view_length")
func viewLength(v: View) -> Int64 { Int64(from: v.length) }

@extern(.C, mangleName: "cb_apply")
func apply(f: IntFn, userdata: Pointer[Int64], x: Int64) -> Int64
@extern(.C, mangleName: "cb_apply_info")
func applyInfo(info: CallbackInfo, x: Int64) -> Int64
@extern(.C, mangleName: "cb_pair")
func pair(f: @extern(.C) (I32x2) -> I32x2) -> Int64
@extern(.C, mangleName: "cb_vec")
func vec(f: @extern(.C) (F32x3) -> F32x3) -> Int64
@extern(.C, mangleName: "cb_apply_mode_info")
func applyModeInfo(info: ModeCallbackInfo, x: Int64) -> Int64
@extern(.C, mangleName: "cb_view")
func passView(f: @extern(.C) (View) -> Int64) -> Int64
@extern(.C, mangleName: "cb_is_null")
func isNull(f: NullableCFunction[IntFn]) -> Int64

@main
func main() -> lang.i64 {
    var offset: Int64 = 40;
    let state = Pointer(to: offset);
    let add: IntFn = addUserdata;
    unsafe {
        println(apply(add, state, 2));
        println(applyInfo(CallbackInfo(callback: NullableCFunction(add), userdata: state), 65));
        offset = 1;
        println(applyInfo(CallbackInfo(callback: NullableCFunction(add), userdata: state), 42));
        println(applyInfo(CallbackInfo(callback: null, userdata: state), 0));
        println(isNull(null));
        println(isNull(NullableCFunction(add)));
        println(vec(reverseVec));
        println(pair(swapPair) == 43);
        let info = ModeCallbackInfo(next: RawPointer.nullPointer(), mode: 3, callback: add, userdata1: state, userdata2: RawPointer.nullPointer());
        println(applyModeInfo(info, 5));
        println(passView(viewLength));
    }
    0
}
