module Main

import std.system.Process

// Bodies are stored as flat struct fields so LLVM keeps everything in registers.
struct Body {
    var x: Float64
    var y: Float64
    var z: Float64
    var vx: Float64
    var vy: Float64
    var vz: Float64
    var mass: Float64
}

func advance(mutating bodies: Slice[Body], dt: Float64) {
    let n: Int64 = 5;
    for i in 0..<n {
        var bi = unsafe bodies(unchecked: i);
        for j in (i + 1)..<n {
            var bj = unsafe bodies(unchecked: j);
            let dx = bi.x - bj.x;
            let dy = bi.y - bj.y;
            let dz = bi.z - bj.z;
            let d2 = dx * dx + dy * dy + dz * dz;
            let mag = dt / (d2 * d2.sqrt());
            let mj = bj.mass * mag;
            bi.vx = bi.vx - dx * mj;
            bi.vy = bi.vy - dy * mj;
            bi.vz = bi.vz - dz * mj;
            let mi = bi.mass * mag;
            bj.vx = bj.vx + dx * mi;
            bj.vy = bj.vy + dy * mi;
            bj.vz = bj.vz + dz * mi;
            unsafe bodies(unchecked: j) = bj;
        }
        bi.x = bi.x + dt * bi.vx;
        bi.y = bi.y + dt * bi.vy;
        bi.z = bi.z + dt * bi.vz;
        unsafe bodies(unchecked: i) = bi;
    }
}

func energy(bodies: Array[Body]) -> Float64 {
    var e = 0.0;
    let n = bodies.count;
    for i in 0..<n {
        let b = bodies(i);
        e = e + 0.5 * b.mass * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz);
        for j in (i + 1)..<n {
            let b2 = bodies(j);
            let dx = b.x - b2.x;
            let dy = b.y - b2.y;
            let dz = b.z - b2.z;
            e = e - b.mass * b2.mass / (dx * dx + dy * dy + dz * dz).sqrt();
        }
    }
    e
}

@main
func main() {
    let args = Process.arguments();
    let steps = if args.count > 1 { Int64(parsing: args(1)) ?? 1000 } else { 1000 };
    let pi: Float64 = 3.141592653589793;
    let solarMass = 4.0 * pi * pi;
    let dpy: Float64 = 365.24;
    var bodies = Array[Body]();
    bodies.append(Body(x: 0.0, y: 0.0, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0, mass: solarMass));
    bodies.append(Body(
        x: 4.84143144246472090e+00, y: -1.16032004402742839e+00, z: -1.03622044471123109e-01,
        vx: 1.66007664274403694e-03 * dpy, vy: 7.69901118419740425e-03 * dpy, vz: -6.90460016972063023e-05 * dpy,
        mass: 9.54791938424326609e-04 * solarMass));
    bodies.append(Body(
        x: 8.34336671824457987e+00, y: 4.12479856412430479e+00, z: -4.03523417114321381e-01,
        vx: -2.76742510726862411e-03 * dpy, vy: 4.99852801234917238e-03 * dpy, vz: 2.30417297573763929e-05 * dpy,
        mass: 2.85885980666130812e-04 * solarMass));
    bodies.append(Body(
        x: 1.28943695621391310e+01, y: -1.51111514016986312e+01, z: -2.23307578892655734e-01,
        vx: 2.96460137564761618e-03 * dpy, vy: 2.37847173959480950e-03 * dpy, vz: -2.96589568540237556e-05 * dpy,
        mass: 4.36624404335156298e-05 * solarMass));
    bodies.append(Body(
        x: 1.53796971148509165e+01, y: -2.59193146099879641e+01, z: 1.79258772950371181e-01,
        vx: 2.68067772490389322e-03 * dpy, vy: 1.62824170038242295e-03 * dpy, vz: -9.51592254519715870e-05 * dpy,
        mass: 5.15138902818349303e-05 * solarMass));

    // Offset momentum so the system's total momentum is zero.
    var px = 0.0; var py = 0.0; var pz = 0.0;
    for b in bodies { px = px + b.vx * b.mass; py = py + b.vy * b.mass; pz = pz + b.vz * b.mass; }
    var sun = bodies(0);
    sun.vx = -px / solarMass; sun.vy = -py / solarMass; sun.vz = -pz / solarMass;
    bodies(0) = sun;

    println("\(energy(bodies):.9)");
    for _ in 0..<steps { advance(bodies.asMutableSlice(), 0.01); }
    println("\(energy(bodies):.9)");
}
