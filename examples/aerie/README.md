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

Controls: WASD to move, Space and Left Shift to rise and fall, arrow keys to
look, Escape to quit. `AERIE_FRAMES=n` quits after n frames.

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
- `src/chunk.ks` — 32³ chunks and the test island.
- `src/mesh.ks` — face culling into packed `FaceInstance`s.
