// Float32 vectors with the layout WGSL's `vec2f`/`vec3f`/`vec4f` read.

module linalg

// ===== Vec2 =====

/// A 2D vector: `vec2<f32>` in WGSL (8 bytes).
public struct Vec2: FFISafe, Equatable {
    public var x: Float32
    public var y: Float32

    public init(x x: Float32, y y: Float32) {
        self.x = x;
        self.y = y;
    }

    /// @name Splat
    /// `(v, v)`.
    public init(splat v: Float32) {
        self.x = v;
        self.y = v;
    }

    public static var zero: Vec2 { Vec2(splat: 0.0) }
    public static var one: Vec2 { Vec2(splat: 1.0) }

    /// The dot product.
    public func dot(other: Vec2) -> Float32 { self.x * other.x + self.y * other.y }

    /// The squared length; cheaper than `length` for comparisons.
    public var lengthSquared: Float32 { self.dot(self) }

    /// The Euclidean length.
    public var length: Float32 { self.lengthSquared.sqrt() }

    /// This vector scaled to length 1, or zero when it is zero.
    public var normalized: Vec2 {
        let len = self.length;
        if len == 0.0 { return Vec2.zero }
        self * (1.0 / len)
    }

    /// The component-wise product.
    public func scaled(by other: Vec2) -> Vec2 { Vec2(x: self.x * other.x, y: self.y * other.y) }

    /// The point `t` of the way from `self` to `other`.
    public func lerp(to other: Vec2, t t: Float32) -> Vec2 { self + (other - self) * t }

    public func isEqual(to other: Vec2) -> Bool { self.x == other.x and self.y == other.y }

    /// `(x, y)` as text.
    public var description: String { "(\(self.x), \(self.y))" }
}

extend Vec2: Addable {
    type Output = Vec2
    public consuming func add(consuming other: Vec2) -> Vec2 { Vec2(x: self.x + other.x, y: self.y + other.y) }
}

extend Vec2: Subtractable {
    type Output = Vec2
    public consuming func subtract(consuming other: Vec2) -> Vec2 { Vec2(x: self.x - other.x, y: self.y - other.y) }
}

extend Vec2: Multipliable[Float32] {
    type Output = Vec2
    public consuming func multiply(consuming s: Float32) -> Vec2 { Vec2(x: self.x * s, y: self.y * s) }
}

extend Vec2: Negatable {
    type Output = Vec2
    public consuming func negate() -> Vec2 { Vec2(x: -self.x, y: -self.y) }
}

// ===== Vec3 =====

/// A 3D vector: `vec3<f32>` in WGSL (12 bytes). In a uniform or storage
/// buffer WGSL aligns a `vec3f` to 16 bytes, so pad after it or use `Vec4`.
public struct Vec3: FFISafe, Equatable {
    public var x: Float32
    public var y: Float32
    public var z: Float32

    public init(x x: Float32, y y: Float32, z z: Float32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    /// @name Splat
    /// `(v, v, v)`.
    public init(splat v: Float32) {
        self.x = v;
        self.y = v;
        self.z = v;
    }

    public static var zero: Vec3 { Vec3(splat: 0.0) }
    public static var one: Vec3 { Vec3(splat: 1.0) }
    public static var unitX: Vec3 { Vec3(x: 1.0, y: 0.0, z: 0.0) }
    public static var unitY: Vec3 { Vec3(x: 0.0, y: 1.0, z: 0.0) }
    public static var unitZ: Vec3 { Vec3(x: 0.0, y: 0.0, z: 1.0) }

    /// The dot product.
    public func dot(other: Vec3) -> Float32 { self.x * other.x + self.y * other.y + self.z * other.z }

    /// The cross product, right-handed.
    public func cross(other: Vec3) -> Vec3 {
        Vec3(
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x
        )
    }

    /// The squared length; cheaper than `length` for comparisons.
    public var lengthSquared: Float32 { self.dot(self) }

    /// The Euclidean length.
    public var length: Float32 { self.lengthSquared.sqrt() }

    /// This vector scaled to length 1, or zero when it is zero.
    public var normalized: Vec3 {
        let len = self.length;
        if len == 0.0 { return Vec3.zero }
        self * (1.0 / len)
    }

    /// The component-wise product.
    public func scaled(by other: Vec3) -> Vec3 { Vec3(x: self.x * other.x, y: self.y * other.y, z: self.z * other.z) }

    /// The point `t` of the way from `self` to `other`.
    public func lerp(to other: Vec3, t t: Float32) -> Vec3 { self + (other - self) * t }

    /// `(x, y, z, w)`.
    public func extended(w w: Float32) -> Vec4 { Vec4(x: self.x, y: self.y, z: self.z, w: w) }

    public func isEqual(to other: Vec3) -> Bool { self.x == other.x and self.y == other.y and self.z == other.z }

    /// `(x, y, z)` as text.
    public var description: String { "(\(self.x), \(self.y), \(self.z))" }
}

extend Vec3: Addable {
    type Output = Vec3
    public consuming func add(consuming o: Vec3) -> Vec3 { Vec3(x: self.x + o.x, y: self.y + o.y, z: self.z + o.z) }
}

extend Vec3: Subtractable {
    type Output = Vec3
    public consuming func subtract(consuming o: Vec3) -> Vec3 { Vec3(x: self.x - o.x, y: self.y - o.y, z: self.z - o.z) }
}

extend Vec3: Multipliable[Float32] {
    type Output = Vec3
    public consuming func multiply(consuming s: Float32) -> Vec3 { Vec3(x: self.x * s, y: self.y * s, z: self.z * s) }
}

extend Vec3: Negatable {
    type Output = Vec3
    public consuming func negate() -> Vec3 { Vec3(x: -self.x, y: -self.y, z: -self.z) }
}

// ===== Vec4 =====

/// A 4D vector: `vec4<f32>` in WGSL (16 bytes), and a homogeneous point or
/// an RGBA color.
public struct Vec4: FFISafe, Equatable {
    public var x: Float32
    public var y: Float32
    public var z: Float32
    public var w: Float32

    public init(x x: Float32, y y: Float32, z z: Float32, w w: Float32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.w = w;
    }

    /// @name Splat
    /// `(v, v, v, v)`.
    public init(splat v: Float32) {
        self.x = v;
        self.y = v;
        self.z = v;
        self.w = v;
    }

    public static var zero: Vec4 { Vec4(splat: 0.0) }
    public static var one: Vec4 { Vec4(splat: 1.0) }

    /// `(x, y, z)`, dropping `w`.
    public var xyz: Vec3 { Vec3(x: self.x, y: self.y, z: self.z) }

    /// The dot product.
    public func dot(other: Vec4) -> Float32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    /// The squared length; cheaper than `length` for comparisons.
    public var lengthSquared: Float32 { self.dot(self) }

    /// The Euclidean length.
    public var length: Float32 { self.lengthSquared.sqrt() }

    /// This vector scaled to length 1, or zero when it is zero.
    public var normalized: Vec4 {
        let len = self.length;
        if len == 0.0 { return Vec4.zero }
        self * (1.0 / len)
    }

    /// The component-wise product.
    public func scaled(by o: Vec4) -> Vec4 { Vec4(x: self.x * o.x, y: self.y * o.y, z: self.z * o.z, w: self.w * o.w) }

    /// The point `t` of the way from `self` to `other`.
    public func lerp(to other: Vec4, t t: Float32) -> Vec4 { self + (other - self) * t }

    public func isEqual(to o: Vec4) -> Bool { self.x == o.x and self.y == o.y and self.z == o.z and self.w == o.w }

    /// `(x, y, z, w)` as text.
    public var description: String { "(\(self.x), \(self.y), \(self.z), \(self.w))" }
}

extend Vec4: Addable {
    type Output = Vec4
    public consuming func add(consuming o: Vec4) -> Vec4 { Vec4(x: self.x + o.x, y: self.y + o.y, z: self.z + o.z, w: self.w + o.w) }
}

extend Vec4: Subtractable {
    type Output = Vec4
    public consuming func subtract(consuming o: Vec4) -> Vec4 { Vec4(x: self.x - o.x, y: self.y - o.y, z: self.z - o.z, w: self.w - o.w) }
}

extend Vec4: Multipliable[Float32] {
    type Output = Vec4
    public consuming func multiply(consuming s: Float32) -> Vec4 { Vec4(x: self.x * s, y: self.y * s, z: self.z * s, w: self.w * s) }
}

extend Vec4: Negatable {
    type Output = Vec4
    public consuming func negate() -> Vec4 { Vec4(x: -self.x, y: -self.y, z: -self.z, w: -self.w) }
}
