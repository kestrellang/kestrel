module Aerie

import linalg.(Vec3)

/// Birds circle over the islands near the player: kestrels over the dream
/// islands, vultures over the lairs. They're scenery: each bird circles a
/// home point above an island, and is dropped once the player leaves it
/// behind, so new ones keep appearing wherever the player goes.
enum BirdKind {
    case Kestrel
    case Vulture

    var isVulture: Bool {
        match self { .Vulture => true, .Kestrel => false }
    }
}

/// A small random number source: a 31-bit LCG, like the noise shuffle.
struct Dice {
    var state: Int64

    /// A number from 0 up to 1.
    mutating func roll() -> Float32 {
        self.state = (self.state * 1103515245 + 12345) % 2147483648;
        Float32(from: self.state) / 2147483648.0
    }

    /// A number from `low` up to `high`.
    mutating func between(low: Float32, high: Float32) -> Float32 {
        low + (high - low) * self.roll()
    }
}

struct Bird {
    var kind: BirdKind
    /// The middle of the circle it flies.
    var home: Vec3
    var radius: Float32
    /// Where it is around the circle, and how fast it goes round, in
    /// radians a second (the sign is the direction).
    var angle: Float32
    var speed: Float32
    /// The wingbeat's phase, and a per-bird offset so a flock doesn't flap
    /// in step.
    var flap: Float32
    var offset: Float32
    /// Kestrels stop to hover, beating fast in place, for `hover` seconds;
    /// `untilHover` counts down to the next time.
    var hover: Float32
    var untilHover: Float32

    /// Where it is now: on its circle, bobbing gently.
    func position(time: Float32) -> Vec3 {
        let bob = (time * 0.6 + self.offset).sin() * 0.8;
        self.home + Vec3(x: self.angle.cos() * self.radius, y: bob, z: self.angle.sin() * self.radius)
    }

    mutating func update(dt: Float32, mutating dice: Dice) {
        match self.kind {
            .Kestrel => { self.kite(dt, dice) },
            .Vulture => { self.soar(dt) }
        }
    }

    /// A kestrel's flight: bursts of quick wingbeats between glides, and
    /// now and then a stop to hover, wings beating fast, holding still
    /// over the ground.
    mutating func kite(dt: Float32, mutating dice: Dice) {
        if self.hover > 0.0 {
            self.hover = self.hover - dt;
            self.flap = self.flap + dt * 22.0;
            return;
        }
        self.untilHover = self.untilHover - dt;
        if self.untilHover <= 0.0 {
            self.hover = dice.between(2.0, 5.0);
            self.untilHover = dice.between(6.0, 14.0);
        }
        self.angle = self.angle + self.speed * dt;
        let beating = (self.offset + self.angle * 1.7).sin() > 0.1;
        self.flap = self.flap + dt * (if beating { 14.0 } else { 0.0 });
    }

    /// A vulture's flight: soaring, with a rare, slow beat.
    mutating func soar(dt: Float32) {
        self.angle = self.angle + self.speed * dt;
        let beating = (self.offset + self.angle * 0.9).sin() > 0.85;
        self.flap = self.flap + dt * (if beating { 5.0 } else { 0.0 });
    }

    /// Drift the circle's middle over `target` and keep above it, once
    /// within 60 blocks: vultures in a lair close in on the player.
    mutating func circle(over target: Vec3, dt dt: Float32) {
        let toward = target - self.home;
        guard toward.x * toward.x + toward.z * toward.z < 60.0 * 60.0 else {
            return;
        }
        let pull = (dt * 0.04).clamp(0.0, 1.0);
        self.home = self.home + Vec3(x: toward.x * pull, y: 0.0, z: toward.z * pull);
        if self.home.y < target.y + 12.0 {
            self.home = self.home + Vec3(x: 0.0, y: dt * 2.0, z: 0.0);
        }
    }

    /// Whether it's more than `reach` blocks from `center` across the
    /// ground at `time`.
    func isFar(center: Vec3, reach reach: Float32, time time: Float32) -> Bool {
        let away = self.position(time) - center;
        away.x * away.x + away.z * away.z > reach * reach
    }

    /// How far the wings are raised, in radians.
    func wingAngle() -> Float32 {
        match self.kind {
            .Kestrel => if self.hover > 0.0 { 0.2 + self.flap.sin() * 0.75 } else { 0.1 + self.flap.sin() * 0.55 },
            // Held in a shallow V, as vultures soar.
            .Vulture => 0.2 + self.flap.sin() * 0.35
        }
    }

    /// The bird's boxes, at `time` seconds.
    func model(time: Float32) -> [ModelBox] {
        let here = self.position(time);
        // Facing along the circle; hovering kestrels hold their heading.
        let direction: Float32 = if self.speed > 0.0 { 1.0 } else { -1.0 };
        let forward = Vec3(x: -self.angle.sin() * direction, y: 0.0, z: self.angle.cos() * direction);
        let flatRight = forward.cross(Vec3.unitY);
        // Banked into the turn: the wing nearer the middle dips.
        let inward = if flatRight.dot(self.home - here) > 0.0 { 1.0 } else { -1.0 };
        let bank: Float32 = if self.hover > 0.0 { 0.0 } else { inward * 0.35 };
        let right = flatRight * bank.cos() - Vec3.unitY * bank.sin();
        let up = Vec3.unitY * bank.cos() + flatRight * bank.sin();
        let pose = Pose(origin: here, right: right, up: up, forward: forward, scale: self.kind.scale);
        self.kind.model(pose, wingAngle: self.wingAngle())
    }
}

extend Pose {
    /// One wing as two boxes hinged at the shoulder and the wrist, raised
    /// `angle` (the outer part flexes further). `side` is 1 for the right wing
    /// and -1 for the left.
    func wing(side: Float32, shoulder shoulder: Vec3, angle angle: Float32, inner inner: Vec3, outer outer: Vec3, layers layers: (Layer, Layer, Layer, Layer)) -> [ModelBox] {
        let (innerTop, outerTop, under, edge) = layers;
        let along = Vec3(x: side * angle.cos(), y: angle.sin(), z: 0.0);
        let lift = Vec3(x: -side * angle.sin(), y: angle.cos(), z: 0.0);
        let tipAngle = angle * 1.5;
        let tipAlong = Vec3(x: side * tipAngle.cos(), y: tipAngle.sin(), z: 0.0);
        let tipLift = Vec3(x: -side * tipAngle.sin(), y: tipAngle.cos(), z: 0.0);
        let wrist = shoulder + along * (inner.x * 2.0);
        // The outer part sweeps back a little.
        let sweep = Vec3(x: 0.0, y: 0.0, z: -outer.z * 0.35);
        [
            self.box(shoulder + along * inner.x, inner, x: along, y: lift, z: Vec3.unitZ, side: edge, top: innerTop, bottom: under),
            self.box(wrist + tipAlong * outer.x + sweep, outer, x: tipAlong, y: tipLift, z: Vec3.unitZ, side: edge, top: outerTop, bottom: under)
        ]
    }
}

extend BirdKind {
    /// How big the model is drawn: kestrels a little over life size, so
    /// they read from a distance.
    var scale: Float32 {
        match self { .Kestrel => 1.3, .Vulture => 1.0 }
    }

    /// The bird's boxes in `pose`, wings raised `wingAngle` radians.
    func model(pose: Pose, wingAngle wingAngle: Float32) -> [ModelBox] {
        match self {
            .Kestrel => BirdKind.kestrelModel(pose, wingAngle),
            .Vulture => BirdKind.vultureModel(pose, wingAngle)
        }
    }

    /// A kestrel: rufous back, cream spotted belly, slate head and tail,
    /// pointed dark wingtips.
    static func kestrelModel(pose: Pose, wingAngle: Float32) -> [ModelBox] {
        var boxes = [
            pose.alignedBox(Vec3(x: 0.0, y: 0.0, z: 0.0), Vec3(x: 0.1, y: 0.09, z: 0.24), side: .KestrelBack, top: .KestrelBack, bottom: .KestrelBelly),
            pose.alignedBox(Vec3(x: 0.0, y: 0.05, z: 0.3), Vec3(x: 0.085, y: 0.085, z: 0.085), side: .KestrelHead, top: .KestrelHead, bottom: .KestrelBelly),
            pose.alignedBox(Vec3(x: 0.0, y: 0.02, z: 0.41), Vec3(x: 0.03, y: 0.03, z: 0.035), side: .Beak, top: .Beak, bottom: .Beak),
            pose.alignedBox(Vec3(x: 0.0, y: 0.01, z: -0.4), Vec3(x: 0.07, y: 0.02, z: 0.17), side: .KestrelTail, top: .KestrelTail, bottom: .KestrelWingUnder)
        ];
        let layers: (Layer, Layer, Layer, Layer) = (.KestrelWing, .KestrelTips, .KestrelWingUnder, .KestrelTips);
        for side in [1.0, -1.0] {
            boxes.append(contentsOf: pose.wing(side, shoulder: Vec3(x: side * 0.08, y: 0.04, z: 0.05), angle: wingAngle,
                inner: Vec3(x: 0.19, y: 0.02, z: 0.12), outer: Vec3(x: 0.18, y: 0.015, z: 0.08), layers: layers));
        }
        boxes
    }

    /// A vulture: sooty and broad-winged, silver flight feathers beneath, a
    /// pale ruff and a bare red head.
    static func vultureModel(pose: Pose, wingAngle: Float32) -> [ModelBox] {
        var boxes = [
            pose.alignedBox(Vec3(x: 0.0, y: 0.0, z: 0.0), Vec3(x: 0.2, y: 0.16, z: 0.44), side: .VultureBody, top: .VultureBody, bottom: .VultureBody),
            pose.alignedBox(Vec3(x: 0.0, y: 0.05, z: 0.44), Vec3(x: 0.15, y: 0.13, z: 0.08), side: .VultureRuff, top: .VultureRuff, bottom: .VultureRuff),
            pose.alignedBox(Vec3(x: 0.0, y: 0.06, z: 0.6), Vec3(x: 0.07, y: 0.07, z: 0.1), side: .VultureHead, top: .VultureHead, bottom: .VultureHead),
            pose.alignedBox(Vec3(x: 0.0, y: 0.02, z: 0.72), Vec3(x: 0.035, y: 0.04, z: 0.04), side: .Beak, top: .Beak, bottom: .Beak),
            pose.alignedBox(Vec3(x: 0.0, y: 0.0, z: -0.6), Vec3(x: 0.17, y: 0.03, z: 0.18), side: .VultureBody, top: .VultureWing, bottom: .VultureWingUnder)
        ];
        let layers: (Layer, Layer, Layer, Layer) = (.VultureWing, .VultureWing, .VultureWingUnder, .VultureBody);
        for side in [1.0, -1.0] {
            boxes.append(contentsOf: pose.wing(side, shoulder: Vec3(x: side * 0.17, y: 0.06, z: 0.05), angle: wingAngle,
                inner: Vec3(x: 0.5, y: 0.03, z: 0.32), outer: Vec3(x: 0.45, y: 0.025, z: 0.26), layers: layers));
        }
        boxes
    }
}

/// The birds near the player.
struct Flock {
    var birds: [Bird]
    var dice: Dice

    init(seed seed: Int64) {
        self.birds = [];
        self.dice = Dice(state: (seed % 2147483648 + 2147483648) % 2147483648);
    }

    /// Move every bird, drop the ones far behind, and try to add one
    /// above an island within `reach` blocks of `center`.
    mutating func update(dt: Float32, center center: Vec3, reach reach: Float32, generator generator: Generator, time time: Float32) {
        let inLair = generator.lairness(center.x, center.z) >= 0.5;
        var kept: [Bird] = [];
        for b in self.birds {
            var bird = b;
            bird.update(dt, self.dice);
            if inLair and bird.kind.isVulture {
                bird.circle(over: center, dt: dt);
            }
            if not bird.isFar(center, reach: reach + 16.0, time: time) {
                kept.append(bird);
            }
        }
        self.birds = kept;
        if self.birds.count < flockSize {
            self.spawn(center, reach, generator);
        }
    }

    /// Put a bird over a random island column around `center`, a kestrel
    /// or a vulture by the realm below. Open sky gets none.
    mutating func spawn(center: Vec3, reach: Float32, generator: Generator) {
        let turn = self.dice.between(0.0, 6.2831853);
        let distance = self.dice.between(reach * 0.2, reach * 0.75);
        let x = center.x + turn.cos() * distance;
        let z = center.z + turn.sin() * distance;
        let column = generator.column(x.floor().toInt64() ?? 0, z.floor().toInt64() ?? 0);
        guard column.isLand else {
            return;
        }
        let ground = Float32(from: column.top);
        let kind: BirdKind = if column.lair { .Vulture } else { .Kestrel };
        let (rise, radius, speed) = match kind {
            .Kestrel => (self.dice.between(7.0, 16.0), self.dice.between(5.0, 11.0), self.dice.between(0.35, 0.6)),
            .Vulture => (self.dice.between(14.0, 26.0), self.dice.between(12.0, 22.0), self.dice.between(0.12, 0.22))
        };
        let spin: Float32 = if self.dice.roll() < 0.5 { 1.0 } else { -1.0 };
        self.birds.append(Bird(
            kind: kind, home: Vec3(x: x, y: ground + rise, z: z), radius: radius,
            angle: self.dice.between(0.0, 6.2831853), speed: speed * spin,
            flap: 0.0, offset: self.dice.between(0.0, 100.0),
            hover: 0.0, untilHover: self.dice.between(3.0, 12.0)
        ));
    }

    func models(time: Float32) -> [ModelBox] {
        var boxes: [ModelBox] = [];
        for bird in self.birds {
            boxes.append(contentsOf: bird.model(time));
        }
        boxes
    }
}

/// How many birds are about at once.
let flockSize: Int64 = 14;
