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
cotton candy gradient (pink horizon, lavender, baby blue overhead), drifting
clouds lit pink on the sun's side, a sea of puffs below the islands, the
sun and its halo, and the moon and twinkling stars at night. Fog fades the
world into that same gradient, sunlight warms to peach at dusk and shadows
are lavender. Every block texture is redrawn at 16x16 (`tools/make_atlas.py`),
the hotbar is a frosted tray with numbered slots and the selected block's
name, and the title screen has an outlined, bobbing name with twinkles.

Controls: WASD to walk, arrow keys to look, Space to jump (hold to glide),
Q to break, E to place, 1 to 9 for the hotbar, T to skip time, P to save, F to fly (Space and Left
Shift to rise and sink), Escape to save and quit, Enter to leave the title screen. `AERIE_FRAMES=n` quits after n
frames.

## Running

Same requirements as `examples/wgpu_scene` (wgpu-native v27 in
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
- `src/font.ks` — the overlay's pixel font, generated by `tools/make_font.py`.
- `src/player.ks` — movement, collision, gliding and block raycasts.
- `src/mesh.ks` — face culling into packed `FaceInstance`s.
