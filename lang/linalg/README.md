# linalg

Float32 vectors and matrices for graphics. Every type is `FFISafe` and laid out the way WGSL reads it, so a `Mat4` or a struct of `Vec3`s can go straight into a GPU buffer (see `examples/wgpu_cube`).

Conventions: right-handed, the camera looks down -Z, and projections map depth to 0…1 (WebGPU, Metal, Vulkan, D3D). Angles are radians; `radians(degrees)` converts.

## Installation

```toml
[dependencies]
linalg = { path = "../../lang/linalg" }
```

## Usage

```kestrel
import linalg.(Vec3, Mat4, radians)

let model = Mat4.rotation(axis: Vec3.unitY, angle: seconds);
let view = Mat4.lookAt(eye: Vec3(x: 0.0, y: 2.0, z: 5.0), target: Vec3.zero, up: Vec3.unitY);
let projection = Mat4.perspective(fovY: radians(60.0), aspect: 16.0 / 9.0, near: 0.1, far: 100.0);
let mvp = projection * view * model;   // applies model first

let p = mvp.transformPoint(Vec3(x: 1.0, y: 0.0, z: 0.0));
```

## Key types

- **Vec2**, **Vec3**, **Vec4**: `x`, `y`, `z`, `w` fields; `init(splat:)`, `zero`, `one` (and `unitX/Y/Z` on `Vec3`).
  - Operators: `a + b`, `a - b`, `v * scalar`, `-v`, `==`.
  - `dot`, `length`, `lengthSquared`, `normalized` (zero stays zero), `scaled(by:)` (component-wise), `lerp(to:t:)`, `description`.
  - `Vec3.cross` (right-handed), `Vec3.extended(w:)`, `Vec4.xyz`.
- **Mat4**: four columns `c0`…`c3`, the layout of WGSL's `mat4x4f`.
  - Builders: `identity`, `translation`, `scaling`, `rotationX/Y/Z`, `rotation(axis:angle:)`, `perspective(fovY:aspect:near:far:)`, `orthographic(left:right:bottom:top:near:far:)`, `lookAt(eye:target:up:)`.
  - `a * b` (apply `b`, then `a`), `m * vec4`, `transformPoint`, `transformDirection`, `transposed`, `determinant`, `inverse` (`None` when singular), `at(row:column:)`.

## GPU layout

`Vec2`, `Vec3`, `Vec4` and `Mat4` are 8, 12, 16 and 64 bytes. In a vertex buffer that matches WGSL exactly. In a uniform or storage buffer WGSL aligns a `vec3f` to 16 bytes, so follow a `Vec3` field with a `Float32` of padding, or use a `Vec4`.
