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
crosshair and E places the hotbar's selected block (1 to 8); each edit
re-meshes the chunks it touches. F toggles free flying. Falling into the
cloud sea puts you back at the spawn island.

Controls: WASD to walk, arrow keys to look, Space to jump (hold to glide),
Q to break, E to place, 1 to 8 for the hotbar, F to fly (Space and Left
Shift to rise and sink), Escape to quit. `AERIE_FRAMES=n` quits after n
frames.

## Running

Same requirements as `examples/wgpu_scene` (wgpu-native v27 in
`WGPU_NATIVE_DIR`, SDL2). Run from this folder so the atlas path resolves:

```sh
WGPU_NATIVE_DIR=~/wgpu-native flock run
```

## Layout

- `src/main.ks` — the desktop driver: an SDL window and loop.
- `src/game.ks` — `Game`, driven one `event` and one `frame` at a time, so a
  browser build can call it from `requestAnimationFrame` instead.
- `src/renderer.ks` — the wgpu side: surface, pipeline, atlas upload.
- `src/shader.ks` — WGSL: expands each packed face into a quad.
- `src/blocks.ks` — block ids, shapes and which atlas layer each face uses.
- `src/chunk.ks` — 32³ chunks, padded chunks for meshing, the test island.
- `src/noise.ks` — seeded 2D gradient noise.
- `src/gen.ks` — the island generator.
- `src/world.ks` — loaded chunks, keyed by position.
- `src/player.ks` — movement, collision, gliding and block raycasts.
- `src/mesh.ks` — face culling into packed `FaceInstance`s.
