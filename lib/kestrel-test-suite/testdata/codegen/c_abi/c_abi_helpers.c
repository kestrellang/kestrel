// C side of the `codegen/c_abi` tests. Each function takes or returns a
// struct by value, so a wrong classification on the Kestrel side shows up
// as a wrong number rather than a crash.
#include <stdint.h>
#include <stdbool.h>

typedef struct { int32_t a, b; } I32x2;          // one INTEGER eightbyte
typedef struct { int64_t a, b; } I64x2;          // two INTEGER eightbytes
typedef struct { double x, y; } F64x2;           // two SSE / an HFA
typedef struct { float x, y; } F32x2;            // one SSE eightbyte
typedef struct { float x, y, z; } F32x3;         // SSE + SSE / an HFA
typedef struct { int32_t i; float f; } Mixed;    // INTEGER (int wins)
typedef struct { double d; int64_t i; } DI;      // SSE + INTEGER
typedef struct { int64_t a, b, c; } Big;         // MEMORY / indirect
typedef struct { uint8_t r, g, b, a; } Rgba;     // 4 bytes, one register

int64_t abi_i32x2_sum(I32x2 v) { return (int64_t)v.a * 10 + v.b; }
I32x2 abi_i32x2_make(int32_t a, int32_t b) { I32x2 v = {a, b}; return v; }
int64_t abi_i64x2_sum(I64x2 v) { return v.a * 100 + v.b; }
I64x2 abi_i64x2_make(int64_t a, int64_t b) { I64x2 v = {a, b}; return v; }
double abi_f64x2_sum(F64x2 v) { return v.x * 10.0 + v.y; }
F64x2 abi_f64x2_make(double x, double y) { F64x2 v = {x, y}; return v; }
float abi_f32x2_sum(F32x2 v) { return v.x * 10.0f + v.y; }
F32x2 abi_f32x2_make(float x, float y) { F32x2 v = {x, y}; return v; }
float abi_f32x3_sum(F32x3 v) { return v.x * 100.0f + v.y * 10.0f + v.z; }
F32x3 abi_f32x3_make(float x, float y, float z) { F32x3 v = {x, y, z}; return v; }
double abi_mixed_sum(Mixed v) { return v.i * 10.0 + v.f; }
Mixed abi_mixed_make(int32_t i, float f) { Mixed v = {i, f}; return v; }
double abi_di_sum(DI v) { return v.d * 10.0 + (double)v.i; }
DI abi_di_make(double d, int64_t i) { DI v = {d, i}; return v; }
int64_t abi_big_sum(Big v) { return v.a * 100 + v.b * 10 + v.c; }
Big abi_big_make(int64_t a, int64_t b, int64_t c) { Big v = {a, b, c}; return v; }
int64_t abi_rgba_sum(Rgba v) { return v.r * 1000000 + v.g * 10000 + v.b * 100 + v.a; }

// Six integer registers used up, so the struct must go on the stack whole.
int64_t abi_after_six(int64_t a, int64_t b, int64_t c, int64_t d, int64_t e, I64x2 v) {
    return a + b + c + d + e + v.a * 1000 + v.b;
}

// 8- and 16-bit arguments: read back at full width.
int64_t abi_u8(uint8_t x) { return x; }
int64_t abi_i8(int8_t x) { return x; }
int64_t abi_u16(uint16_t x) { return x; }
int64_t abi_i16(int16_t x) { return x; }
int64_t abi_bool(bool x) { return x ? 1 : 0; }
uint8_t abi_ret_u8(void) { return 200; }
int8_t abi_ret_i8(void) { return -5; }

// Pointers from reference params.
int64_t abi_read_ptr(const I64x2 *v) { return v->a * 100 + v->b; }
void abi_write_ptr(I64x2 *v) { v->a += 1; v->b += 2; }
