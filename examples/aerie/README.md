# Aerie

A voxel sandbox on floating islands above a cloud sea, drawn with `lang/wgpu`.
The block art is original: `tools/make_atlas.py` draws every 16x16 tile from a
fixed seed and a small pastel palette into `assets/blocks.png`, and the game
loads it with `lang/images`.

**Milestone 1** (this version) draws a hand-built test island from the atlas:
face-culled blocks, one instanced quad per visible face, a mipmapped texture
array, 4x MSAA and fog, with a fly camera.

**Milestone 2** adds ambient occlusion: each face corner darkens by how many
solid blocks touch it, and quads split along the brighter diagonal so the
shading has no crease. `AERIE_BENCH=n` times chunk generation and meshing
without a window.

**Milestone 3** replaces the test island with an endless archipelago. A
low-frequency noise mask decides where land is; each island floats at its own
altitude with a keel underneath. Chunk columns (four 32³ chunks, 128 blocks
tall) generate and mesh around the player within a few milliseconds per
frame, nearest first, and are dropped once far away. Each chunk is meshed
with a one-block border from its neighbors, so there are no seams, and
chunks outside the camera's view are skipped.

**Milestone 4** makes it playable. You walk with gravity and collision,
jump, and hold jump while falling to open the wing-cloak and glide: you sink
slowly and sail forward at 10 blocks a second. Q breaks the block under the
crosshair and E places the hotbar's selected block (1 to 9); each edit
re-meshes the chunks it touches. F toggles free flying. Falling into the
cloud sea puts you back at the spawn island.

**Milestone 5** adds light and time. The sun crosses the sky over an
eight-minute day; a depth-only shadow pass renders the world from the sun
into a 2048² shadow map that follows the player, and the sky, fog and
ambient light fade from dawn to a dark-blue night. Glowcaps give off a cool
light: each chunk carries the glowcaps near it in its own bind group, and
the shader adds their falloff, so they light up the ground at night. T skips
ahead a quarter day, and `AERIE_TIME=0..1` sets the starting time (0 dawn,
0.25 noon, 0.5 dusk, 0.75 midnight).

**Milestone 6** saves the world and adds some polish. Only chunks you changed are
kept; everything else is regenerated from the seed. Escape saves and quits
and P saves on the spot, to `world.aerie` (zlib-compressed with
`lang/deflate`), or the path in `AERIE_SAVE`; `AERIE_SAVE=none` turns
saving off. Changed chunks that stream out of range are held until they
come back or the world is saved. Cloudglass is see-through: glass faces
are meshed separately and blended after the solid world, far chunks first.
A title screen turns over the world until you press Enter (`AERIE_PLAY=1`
skips it), and the overlay draws text with a built-in 3x5 pixel font.

**Milestone 7** runs Aerie in a browser. `web/build.sh` compiles it to
wasm64 with Emscripten, using browser WebGPU (Dawn's `emdawnwebgpu`) and
SDL2's Emscripten port; saves go to IndexedDB. The game code is shared;
`src/platform.ks` holds the few desktop and web differences.

**Milestone 8** makes it dreamy. The sky is drawn as its own pass: a
soft pastel gradient (rose-cream horizon, baby blue overhead), drifting
clouds warmed on the sun's side, a sea of puffs below the islands, the
sun and its halo, and the moon and twinkling stars at night. Fog fades the
world into that same gradient, sunlight warms to peach at dusk and shadows
are cool blue-grey. Every block texture is redrawn at 16x16 (`tools/make_atlas.py`),
and the interface comes from a generated atlas (`tools/make_ui.py`,
`assets/ui.png`): text is set in Inter (SIL Open Font License) with upper and
lowercase, the hotbar is a frosted tray of rounded slots holding little 3D
blocks, and the title screen has an outlined, bobbing name with twinkles.
The mouse looks around once captured (click, or Enter on the title).

About half the sky belongs to evil lairs. A low-frequency theme noise
(`Generator.lairness`) splits the world into dream and lair regions; lair
islands grow scorched loam over ashrock with spiked keels, dead trees with
sparse gloomleaf, hexblooms in place of glowcaps, embercore veins that glow
red, and the odd darkbrick spire with ember windows and a brazier on top.
As you fly into a lair the sky, sun, clouds and ambient light ease toward
a crimson dusk (`Game.gloom`, carried to the shaders in `sunColor.w`). The
border follows the noise, not the islands, so an island can straddle it.

Birds fly over the islands (`src/birds.ks`): kestrels over the dream
islands, circling and stopping to hover on fast wingbeats, and broad-winged
vultures soaring over the lairs, where they drift over to circle above
you. Each is a handful of boxes (`ModelBox` in `src/model.ks`) drawn by a
model pipeline that takes boxes at any angle, so wings hinge at the
shoulder and wrist, and the birds cast shadows. Breaking a block takes
holding: cracks spread over it for its `Block.hardness` in seconds (a
glowcap pops at once, skyiron takes two and a half), drawn by the same
pipeline.

Controls: WASD to walk, the mouse or arrow keys to look, Space to jump (hold to glide),
hold left click or Q to break, right click or E to place, the wheel or 1 to 9 for the hotbar, T to skip time, P to save, - and = to shrink or grow the view distance, F to fly (Space and Left
Shift to rise and sink), Escape to free the mouse (and again to save and quit), Enter or a click to leave the title screen. `AERIE_FRAMES=n` quits after n
frames. `AERIE_VIEW=n` sets the view distance to n chunks (2 to 16, default 5); fog
only covers the last quarter of it, so chunks fade in at the edge.

## Running

Same requirements as `examples/wgpu_scene` (wgpu-native v27.0.2.0 in
`WGPU_NATIVE_DIR`, SDL2). Run from this folder so the atlas path resolves:

```sh
WGPU_NATIVE_DIR=~/wgpu-native flock run
```

### In a browser

Needs Emscripten 4 or later and a browser with WebGPU and wasm64
(Memory64), such as a recent Chrome. From the repository root:

```sh
source ~/emsdk/emsdk_env.sh
examples/aerie/web/build.sh
python3 -m http.server -d examples/aerie/web/out 8000   # open localhost:8000
```

## Layout

- `src/main.ks` — the driver: an SDL window and loop, on the desktop and in a browser.
- `src/platform.ks` — what differs between desktop and browser: the surface, save path, frame end.
- `web/` — the browser build script, page shell and a wasm64 workaround.
- `src/game.ks` — `Game`, driven one `event` and one `frame` at a time, so a
  browser build can call it from `requestAnimationFrame` instead.
- `src/renderer.ks` — the wgpu side: surface, shadow and main passes, HUD overlay, daylight.
- `src/shader.ks` — WGSL: expands each packed face into a quad, sun shadows, glow, fog, and the sky.
- `src/blocks.ks` — block ids, shapes and which atlas layer each face uses.
- `src/chunk.ks` — 32³ chunks, padded chunks for meshing, the test island.
- `src/noise.ks` — seeded 2D gradient noise.
- `src/gen.ks` — the island generator.
- `src/world.ks` — loaded chunks, keyed by position, and changed ones kept for saving.
- `src/save.ks` — the save format: encode and decode as bytes.
- `src/hud.ks` — the overlay: text, the hotbar and the title screen.
- `src/ui.ks` — the interface atlas's layout and glyph widths, generated by `tools/make_ui.py`.
- `src/birds.ks` — kestrels and vultures: flight, spawning near the player and their models.
- `src/model.ks` — `ModelBox`, a textured box at any angle, and `Pose` to build models from them.
- `src/player.ks` — movement, collision, gliding and block raycasts.
- `src/mesh.ks` — face culling into packed `FaceInstance`s.
