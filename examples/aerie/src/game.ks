module Aerie

import wgpu.(WgpuError)
import sdl.(Event, Key)
import linalg.(Vec3, Mat4)

/// Which movement keys are held.
struct Controls {
    var forward: Bool = false
    var back: Bool = false
    var left: Bool = false
    var right: Bool = false
    var up: Bool = false
    var down: Bool = false
    var turnLeft: Bool = false
    var turnRight: Bool = false
    var lookUp: Bool = false
    var lookDown: Bool = false

    init() {}

    mutating func setKey(key: Key, held held: Bool) {
        match key {
            .W => { self.forward = held },
            .S => { self.back = held },
            .A => { self.left = held },
            .D => { self.right = held },
            .Space => { self.up = held },
            .LeftShift => { self.down = held },
            .LeftArrow => { self.turnLeft = held },
            .RightArrow => { self.turnRight = held },
            .UpArrow => { self.lookUp = held },
            .DownArrow => { self.lookDown = held },
            _ => {}
        }
    }
}

func axis(positive: Bool, negative: Bool) -> Float32 {
    (if positive { 1.0 } else { 0.0 }) - (if negative { 1.0 } else { 0.0 })
}

/// A free-flying camera at `position`, turned `yaw` radians from looking
/// down -z and tilted `pitch` radians up. Milestone 4 replaces flying with
/// walking and gliding.
struct Camera {
    var position: Vec3
    var yaw: Float32
    var pitch: Float32

    var forward: Vec3 {
        Vec3(
            x: self.yaw.sin() * self.pitch.cos(),
            y: self.pitch.sin(),
            z: -self.yaw.cos() * self.pitch.cos()
        )
    }

    mutating func update(controls: Controls, dt dt: Float32) {
        self.yaw = self.yaw + axis(controls.turnRight, controls.turnLeft) * 1.8 * dt;
        let limit = 1.5;
        self.pitch = (self.pitch + axis(controls.lookUp, controls.lookDown) * 1.4 * dt).clamp(-limit, limit);
        let ahead = Vec3(x: self.yaw.sin(), y: 0.0, z: -self.yaw.cos());
        let side = ahead.cross(Vec3.unitY);
        let move = ahead * axis(controls.forward, controls.back)
            + side * axis(controls.right, controls.left)
            + Vec3.unitY * axis(controls.up, controls.down);
        self.position = self.position + move * (10.0 * dt);
    }

    func view() -> Mat4 {
        Mat4.lookAt(eye: self.position, target: self.position + self.forward, up: Vec3.unitY)
    }
}

/// The game, driven one event and one frame at a time. Nothing here owns
/// the main loop, so a browser build can call `frame` from
/// `requestAnimationFrame` instead of a `while` loop.
struct Game: not Copyable {
    var renderer: Renderer
    var camera: Camera
    var controls: Controls
    var seconds: Float32
    var running: Bool

    init(consuming renderer: Renderer) {
        self.renderer = renderer;
        self.camera = Camera(position: Vec3(x: 33.0, y: 29.0, z: 38.0), yaw: -0.6, pitch: -0.3);
        self.controls = Controls();
        self.seconds = 0.0;
        self.running = true;
        let island = testIsland();
        let faces = meshChunk(island);
        println("test island: \(faces.count) faces");
        self.renderer.upload(faces);
    }

    mutating func event(event: Event) {
        match event {
            .Quit => { self.running = false },
            .KeyDown(.Escape) => { self.running = false },
            .KeyDown(key) => { self.controls.setKey(key, held: true) },
            .KeyUp(key) => { self.controls.setKey(key, held: false) },
            _ => {}
        }
    }

    mutating func frame(dt: Float32) -> () throws WgpuError {
        self.seconds = self.seconds + dt;
        self.camera.update(self.controls, dt: dt);
        // A low sun that slowly circles, for long shadows on the island.
        let angle = 0.6 + self.seconds * 0.05;
        let sun = Vec3(x: angle.cos(), y: -0.7, z: angle.sin()).normalized;
        try self.renderer.draw(self.camera.view(), eye: self.camera.position, sun: sun);
        .Ok(())
    }
}
