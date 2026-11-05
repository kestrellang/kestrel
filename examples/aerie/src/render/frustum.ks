module Aerie

import linalg.(Vec3, Vec4, Mat4)

/// What a view-projection can see: six planes, each (a, b, c, d) with the
/// inside where ax + by + cz + d >= 0.
struct Frustum {
    var planes: [Vec4]

    init(viewProj m: Mat4) {
        // Row i of the column-major matrix.
        let r0 = Vec4(x: m.c0.x, y: m.c1.x, z: m.c2.x, w: m.c3.x);
        let r1 = Vec4(x: m.c0.y, y: m.c1.y, z: m.c2.y, w: m.c3.y);
        let r2 = Vec4(x: m.c0.z, y: m.c1.z, z: m.c2.z, w: m.c3.z);
        let r3 = Vec4(x: m.c0.w, y: m.c1.w, z: m.c2.w, w: m.c3.w);
        // WebGPU clip space has 0 <= z <= w.
        self.planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2];
    }

    /// Whether any of the box from `low` to `high` is inside every plane.
    func contains(low: Vec3, high: Vec3) -> Bool {
        for p in self.planes {
            // The box corner furthest along the plane's normal.
            let x = if p.x >= 0.0 { high.x } else { low.x };
            let y = if p.y >= 0.0 { high.y } else { low.y };
            let z = if p.z >= 0.0 { high.z } else { low.z };
            if p.x * x + p.y * y + p.z * z + p.w < 0.0 {
                return false;
            }
        }
        true
    }
}
