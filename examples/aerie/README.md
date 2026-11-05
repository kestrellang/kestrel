# Kestrelcraft

Kestrelcraft (developed here as Aerie: the folder, module and binary keep
that name) is a voxel sandbox on floating islands above a cloud sea, drawn with `lang/wgpu`.
The block art is original: `tools/make_atlas.py` draws every 16x16 tile from a
fixed seed and a small pastel palette into `assets/blocks.png`, and the game
loads it with `lang/images`.

**Milestone 1** drew a hand-built test island from the atlas:
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

Soaring works like a glider: a double jump launches you about twelve
blocks up and opens the wing-cloak at the top. Your path follows where you
look; diving trades height for speed and climbing spends it, and below
stall speed you sink. Each jump while soaring boosts you along your view,
and the view widens as you speed up. Landing folds the cloak.

**Milestone 8** makes it dreamy. The sky is drawn as its own pass: a
soft pastel gradient (rose-cream horizon, baby blue overhead), drifting
clouds warmed on the sun's side, a sea of puffs below the islands, the
sun and its halo, and the moon and twinkling stars at night. Fog fades the
world into that same gradient, sunlight warms to peach at dusk and shadows
are cool blue-grey. Every block texture is redrawn at 16x16 (`tools/make_atlas.py`),
and the interface comes from a generated atlas (`tools/make_ui.py`,
`assets/ui.png`): text is set in our own pixel font (`tools/pixel_font.py`),
upper and lowercase, drawn crisp at whole-pixel sizes. The interface is pixel art to match the blocks: a wooden tray of
stone slots holding little 3D blocks (gold for the selected one), wooden
boards behind text, a warm wooden button on the title screen, and a logo
whose every pixel is a driftstone block, mossy where it's open to the sky.
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

Controls: WASD to walk, the mouse to look, Space to jump, and Space again in
midair to launch up and soar on the wing-cloak (look down to dive and gain
speed, up to climb, Space to boost, Shift to fold the cloak),
hold left click to break, right click to place, the wheel or 1 to 9 for the
hotbar, F to fly (Space and Left Shift to rise and sink), Z to show frames per
second, Escape to free the mouse (and again to save and quit), Enter or a
click to leave the title screen. The world also saves itself every minute.
`AERIE_FRAMES=n` quits after n
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

How the code is written (object-oriented, flat control flow, one source of
truth) is in [`AGENTS.md`](AGENTS.md).

- `src/main.ks` — the driver: an SDL window and loop, on the desktop and in a browser.
- `src/platform.ks` — what differs between desktop and browser: the surface, save path, frame end.
- `src/savefile.ks` — `SaveFile`: reading and writing the compressed save.
- `src/bench.ks` — `AERIE_BENCH`: timing generation and meshing.
- `src/game/` — play itself:
  - `game.ks` — `Game`, driven one `event` and one `frame` at a time, so a
    browser build can call it from `requestAnimationFrame` instead.
  - `settings.ks` — `Settings`, everything read from the environment.
  - `controls.ks` — which keys and buttons are held or pressed.
  - `player.ks` — the camera, movement, collision and gliding.
  - `hotbar.ks` — the blocks you can place and which is picked.
  - `editing.ks` — breaking (with cracks) and placing blocks.
  - `sky.ks` — the time of day, the lairs' gloom, and autosave timing.
  - `birds.ks` — kestrels and vultures: flight, spawning near the player and their models.
- `src/world/` — the blocks and the islands:
  - `blocks.ks` — block ids, shapes and which atlas layer each face uses.
  - `chunk.ks` — 32³ chunks, and padded chunks for meshing.
  - `noise.ks` — seeded 2D gradient noise.
  - `gen.ks` — the island generator: columns, trees, towers and the spawn point.
  - `world.ks` — loaded chunks, keyed by position, changed ones kept for saving, and lights.
  - `raycast.ks` — the block you're looking at, and what blocks the player.
  - `stream.ks` — generating, meshing and dropping chunk columns around the player.
  - `save.ks` — the save format: encode and decode as bytes.
- `src/render/` — the GPU side:
  - `renderer.ks` — `Renderer`: the surface, uniforms and the frame's passes.
  - `chunks.ks` — `ChunkPass`: every chunk's faces in one shared buffer, culled and drawn in runs.
  - `mesh.ks` — face culling and ambient occlusion into packed `FaceInstance`s.
  - `model.ks` — `ModelBox`, `Pose` and `ModelPass`: boxes at any angle, for birds and cracks.
  - `overlay.ks` — `OverlayQuad` and `OverlayPass`: the 2D interface.
  - `daylight.ks` — the sun, sky colors and shadow view for a time of day.
  - `frustum.ks`, `atlas.ks` — view culling, and loading texture atlases.
  - `shader.ks` — loads the WGSL in `assets/shaders/`, after a header of constants the game decides.
- `src/ui/` — the interface:
  - `hud.ks`, `title.ks` — `Hud`: the crosshair and hotbar, and the title screen.
  - `label.ks` — `Label`: text in the pixel font, shadowed, centered or on a board.
  - `quads.ks` — building overlay quads from sprites and colors.
  - `readouts.ks` — the FPS counter and notices in the corner.
  - `ui.ks` — the interface atlas's layout and glyph widths, generated by `tools/make_ui.py`.
- `assets/shaders/` — `blocks.wgsl` (faces, shadows, glow, fog, the sky and models),
  `overlay.wgsl` (the 2D interface) and `surface.wgsl` (the sRGB step every shader's output goes through).
- `tools/` — generators for the block atlas, the interface atlas and the pixel font.
- `web/` — the browser build script, page shell and a wasm64 workaround.
