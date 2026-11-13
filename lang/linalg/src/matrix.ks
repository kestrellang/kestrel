// A column-major Float32 4x4 matrix with the layout WGSL's `mat4x4f` reads,
// and the transforms a 3D renderer needs. Conventions: right-handed, camera
// looking down -Z, clip-space depth 0 to 1 (WebGPU, Metal, Vulkan, D3D).

module linalg

/// The angle `degrees` in radians.
public func radians(degrees: Float32) -> Float32 { degrees * Float32.pi / 180.0 }

/// A 4x4 matrix stored as four columns: `mat4x4<f32>` in WGSL (64 bytes).
/// `m * v` transforms the column vector `v`; `a * b` applies `b` first.
public struct Mat4: FFISafe, Equatable {
    public var c0: Vec4
    public var c1: Vec4
    public var c2: Vec4
    public var c3: Vec4

    /// @name From Columns
    public init(c0: Vec4, c1: Vec4, c2: Vec4, c3: Vec4) {
        self.c0 = c0;
        self.c1 = c1;
        self.c2 = c2;
        self.c3 = c3;
    }

    /// The identity matrix.
    public static var identity: Mat4 {
        Mat4(
            Vec4(x: 1.0, y: 0.0, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 1.0, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 1.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
        )
    }

    /// Moves points by `offset`.
    public static func translation(offset: Vec3) -> Mat4 {
        var m = Mat4.identity;
        m.c3 = offset.extended(w: 1.0);
        m
    }

    /// Scales each axis by the matching component of `factors`.
    public static func scaling(factors: Vec3) -> Mat4 {
        Mat4(
            Vec4(x: factors.x, y: 0.0, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: factors.y, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: factors.z, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
        )
    }

    /// Rotates `angle` radians about the X axis (counter-clockwise looking
    /// down the axis toward the origin).
    public static func rotationX(angle: Float32) -> Mat4 {
        let s = angle.sin();
        let c = angle.cos();
        Mat4(
            Vec4(x: 1.0, y: 0.0, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: c, z: s, w: 0.0),
            Vec4(x: 0.0, y: -s, z: c, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
        )
    }

    /// Rotates `angle` radians about the Y axis.
    public static func rotationY(angle: Float32) -> Mat4 {
        let s = angle.sin();
        let c = angle.cos();
        Mat4(
            Vec4(x: c, y: 0.0, z: -s, w: 0.0),
            Vec4(x: 0.0, y: 1.0, z: 0.0, w: 0.0),
            Vec4(x: s, y: 0.0, z: c, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
        )
    }

    /// Rotates `angle` radians about the Z axis.
    public static func rotationZ(angle: Float32) -> Mat4 {
        let s = angle.sin();
        let c = angle.cos();
        Mat4(
            Vec4(x: c, y: s, z: 0.0, w: 0.0),
            Vec4(x: -s, y: c, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 1.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
        )
    }

    /// Rotates `angle` radians about `axis` (normalized here).
    public static func rotation(axis axis: Vec3, angle angle: Float32) -> Mat4 {
        let a = axis.normalized;
        let s = angle.sin();
        let c = angle.cos();
        let t = 1.0 - c;
        Mat4(
            Vec4(x: t * a.x * a.x + c, y: t * a.x * a.y + s * a.z, z: t * a.x * a.z - s * a.y, w: 0.0),
            Vec4(x: t * a.x * a.y - s * a.z, y: t * a.y * a.y + c, z: t * a.y * a.z + s * a.x, w: 0.0),
            Vec4(x: t * a.x * a.z + s * a.y, y: t * a.y * a.z - s * a.x, z: t * a.z * a.z + c, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: 0.0, w: 1.0)
        )
    }

    /// A perspective projection: vertical field of view `fovY` radians,
    /// width/height `aspect`, and the visible depth range `near` to `far`
    /// mapped to clip depth 0 to 1.
    public static func perspective(fovY fovY: Float32, aspect aspect: Float32, near near: Float32, far far: Float32) -> Mat4 {
        let f = 1.0 / (fovY * 0.5).tan();
        let range = 1.0 / (near - far);
        Mat4(
            Vec4(x: f / aspect, y: 0.0, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: f, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: far * range, w: -1.0),
            Vec4(x: 0.0, y: 0.0, z: near * far * range, w: 0.0)
        )
    }

    /// An orthographic projection of the box `left…right`, `bottom…top`,
    /// `near…far` (distances in front of the camera) onto clip space.
    public static func orthographic(left left: Float32, right right: Float32, bottom bottom: Float32, top top: Float32, near near: Float32, far far: Float32) -> Mat4 {
        let w = 1.0 / (right - left);
        let h = 1.0 / (top - bottom);
        let d = 1.0 / (near - far);
        Mat4(
            Vec4(x: 2.0 * w, y: 0.0, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 2.0 * h, z: 0.0, w: 0.0),
            Vec4(x: 0.0, y: 0.0, z: d, w: 0.0),
            Vec4(x: -(right + left) * w, y: -(top + bottom) * h, z: near * d, w: 1.0)
        )
    }

    /// A view matrix for a camera at `eye` looking at `target`, with `up`
    /// roughly up.
    public static func lookAt(eye eye: Vec3, target target: Vec3, up up: Vec3) -> Mat4 {
        let f = (target - eye).normalized;
        let s = f.cross(up).normalized;
        let u = s.cross(f);
        Mat4(
            Vec4(x: s.x, y: u.x, z: -f.x, w: 0.0),
            Vec4(x: s.y, y: u.y, z: -f.y, w: 0.0),
            Vec4(x: s.z, y: u.z, z: -f.z, w: 0.0),
            Vec4(x: -s.dot(eye), y: -u.dot(eye), z: f.dot(eye), w: 1.0)
        )
    }

    /// Row `row`, column `column`, both 0 to 3.
    public func at(row row: Int64, column column: Int64) -> Float32 {
        let c = match column {
            0 => self.c0,
            1 => self.c1,
            2 => self.c2,
            _ => self.c3
        };
        match row {
            0 => c.x,
            1 => c.y,
            2 => c.z,
            _ => c.w
        }
    }

    /// Rows as columns.
    public var transposed: Mat4 {
        Mat4(
            Vec4(x: self.c0.x, y: self.c1.x, z: self.c2.x, w: self.c3.x),
            Vec4(x: self.c0.y, y: self.c1.y, z: self.c2.y, w: self.c3.y),
            Vec4(x: self.c0.z, y: self.c1.z, z: self.c2.z, w: self.c3.z),
            Vec4(x: self.c0.w, y: self.c1.w, z: self.c2.w, w: self.c3.w)
        )
    }

    /// The determinant.
    public var determinant: Float32 {
        let m = Minors(self);
        m.s0 * m.c5 - m.s1 * m.c4 + m.s2 * m.c3 + m.s3 * m.c2 - m.s4 * m.c1 + m.s5 * m.c0
    }

    /// The inverse, or `None` when the matrix is singular.
    public var inverse: Mat4? {
        let m = Minors(self);
        let det = m.s0 * m.c5 - m.s1 * m.c4 + m.s2 * m.c3 + m.s3 * m.c2 - m.s4 * m.c1 + m.s5 * m.c0;
        if det == 0.0 {
            return .None
        }
        let k = 1.0 / det;
        let a = self.c0;
        let b = self.c1;
        let c = self.c2;
        let d = self.c3;
        // Columns of the adjugate, from the 2x2 minors (see `Minors`).
        .Some(Mat4(
            Vec4(
                x: (b.y * m.c5 - c.y * m.c4 + d.y * m.c3) * k,
                y: (-a.y * m.c5 + c.y * m.c2 - d.y * m.c1) * k,
                z: (a.y * m.c4 - b.y * m.c2 + d.y * m.c0) * k,
                w: (-a.y * m.c3 + b.y * m.c1 - c.y * m.c0) * k
            ),
            Vec4(
                x: (-b.x * m.c5 + c.x * m.c4 - d.x * m.c3) * k,
                y: (a.x * m.c5 - c.x * m.c2 + d.x * m.c1) * k,
                z: (-a.x * m.c4 + b.x * m.c2 - d.x * m.c0) * k,
                w: (a.x * m.c3 - b.x * m.c1 + c.x * m.c0) * k
            ),
            Vec4(
                x: (b.w * m.s5 - c.w * m.s4 + d.w * m.s3) * k,
                y: (-a.w * m.s5 + c.w * m.s2 - d.w * m.s1) * k,
                z: (a.w * m.s4 - b.w * m.s2 + d.w * m.s0) * k,
                w: (-a.w * m.s3 + b.w * m.s1 - c.w * m.s0) * k
            ),
            Vec4(
                x: (-b.z * m.s5 + c.z * m.s4 - d.z * m.s3) * k,
                y: (a.z * m.s5 - c.z * m.s2 + d.z * m.s1) * k,
                z: (-a.z * m.s4 + b.z * m.s2 - d.z * m.s0) * k,
                w: (a.z * m.s3 - b.z * m.s1 + c.z * m.s0) * k
            )
        ))
    }

    /// `point` moved by this matrix (as `(x, y, z, 1)`, divided by the
    /// resulting `w`).
    public func transformPoint(point: Vec3) -> Vec3 {
        let p = self * point.extended(w: 1.0);
        p.xyz * (1.0 / p.w)
    }

    /// `direction` turned by this matrix, ignoring translation.
    public func transformDirection(direction: Vec3) -> Vec3 {
        (self * direction.extended(w: 0.0)).xyz
    }

    public func isEqual(to o: Mat4) -> Bool {
        self.c0 == o.c0 and self.c1 == o.c1 and self.c2 == o.c2 and self.c3 == o.c3
    }

    /// The four rows as text.
    public var description: String {
        let t = self.transposed;
        "[\(t.c0.description), \(t.c1.description), \(t.c2.description), \(t.c3.description)]"
    }
}

/// The 2x2 minors a 4x4 determinant and inverse share. With columns
/// `a, b, c, d`, `s*` pair rows 0/1 (x, y) and `c*` pair rows 2/3 (z, w).
struct Minors {
    var s0: Float32
    var s1: Float32
    var s2: Float32
    var s3: Float32
    var s4: Float32
    var s5: Float32
    var c0: Float32
    var c1: Float32
    var c2: Float32
    var c3: Float32
    var c4: Float32
    var c5: Float32

    init(m: Mat4) {
        let a = m.c0;
        let b = m.c1;
        let c = m.c2;
        let d = m.c3;
        self.s0 = a.x * b.y - b.x * a.y;
        self.s1 = a.x * c.y - c.x * a.y;
        self.s2 = a.x * d.y - d.x * a.y;
        self.s3 = b.x * c.y - c.x * b.y;
        self.s4 = b.x * d.y - d.x * b.y;
        self.s5 = c.x * d.y - d.x * c.y;
        self.c5 = c.z * d.w - d.z * c.w;
        self.c4 = b.z * d.w - d.z * b.w;
        self.c3 = b.z * c.w - c.z * b.w;
        self.c2 = a.z * d.w - d.z * a.w;
        self.c1 = a.z * c.w - c.z * a.w;
        self.c0 = a.z * b.w - b.z * a.w;
    }
}

extend Mat4: Multipliable[Mat4] {
    type Output = Mat4
    public static var one: Mat4 { Mat4.identity }

    /// `self` applied after `other`.
    public consuming func multiply(consuming other: Mat4) -> Mat4 {
        Mat4(self * other.c0, self * other.c1, self * other.c2, self * other.c3)
    }
}

extend Mat4: Multipliable[Vec4] {
    type Output = Vec4
    public static var one: Mat4 { Mat4.identity }

    public consuming func multiply(consuming v: Vec4) -> Vec4 {
        self.c0 * v.x + self.c1 * v.y + self.c2 * v.z + self.c3 * v.w
    }
}
