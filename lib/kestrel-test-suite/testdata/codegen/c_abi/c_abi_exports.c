// C side of `export_called_from_c.ks`: calls the Kestrel `@export`s by
// symbol and scores one digit per correct result.
#include <stdint.h>

typedef struct { int32_t a, b; } I32x2;          // one INTEGER eightbyte
typedef struct { int64_t a, b; } I64x2;          // two INTEGER eightbytes
typedef struct { double x, y; } F64x2;           // two SSE / an HFA
typedef struct { float x, y; } F32x2;            // one SSE eightbyte
typedef struct { float x, y, z; } F32x3;         // SSE + SSE / an HFA
typedef struct { int32_t i; float f; } Mixed;    // INTEGER (int wins)
typedef struct { double d; int64_t i; } DI;      // SSE + INTEGER
typedef struct { int64_t a, b, c; } Big;         // MEMORY / indirect
typedef struct { uint8_t r, g, b, a; } Rgba;     // 4 bytes, one register

// Kestrel `@export`s, called back from C.
extern I32x2 kestrel_i32x2_swap(I32x2 v);
extern F32x3 kestrel_f32x3_scale(F32x3 v, float k);
extern Big kestrel_big_rotate(Big v);
extern DI kestrel_di_add(DI v, int64_t n);
extern int64_t kestrel_add_u8(uint8_t a, int8_t b);

int64_t abi_call_exports(void) {
    I32x2 s = kestrel_i32x2_swap((I32x2){1, 2});
    F32x3 f = kestrel_f32x3_scale((F32x3){1.0f, 2.0f, 3.0f}, 2.0f);
    Big b = kestrel_big_rotate((Big){1, 2, 3});
    DI d = kestrel_di_add((DI){1.5, 2}, 3);
    int64_t u = kestrel_add_u8(250, -3);
    int64_t ok = 0;
    ok += (s.a == 2 && s.b == 1);
    ok += (f.x == 2.0f && f.y == 4.0f && f.z == 6.0f) * 10;
    ok += (b.a == 2 && b.b == 3 && b.c == 1) * 100;
    ok += (d.d == 1.5 && d.i == 5) * 1000;
    ok += (u == 247) * 10000;
    return ok;
}
