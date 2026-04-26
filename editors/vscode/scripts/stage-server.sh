#!/usr/bin/env bash
# Build kestrel-lsp in release mode and copy the binary into
# `editors/vscode/server/<platform>-<arch>/` so the bundled-binary
# resolution path in `extension.ts` finds it.
#
# Used both by local development (`npm run server:build`) and by CI
# release packaging — CI runs this on each target host so the resulting
# `server/` tree matches the matrix that gets bundled into per-platform
# .vsix files.
#
# The platform/arch pair is computed from `node -p` so it matches what
# the extension's `process.platform`/`process.arch` returns at runtime.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ext_root="$(cd "$here/.." && pwd)"
repo_root="$(cd "$ext_root/../.." && pwd)"

platform="$(node -p 'process.platform')"
arch="$(node -p 'process.arch')"
exe_name="kestrel-lsp"
if [[ "$platform" == "win32" ]]; then
  exe_name="kestrel-lsp.exe"
fi

target_dir="$ext_root/server/$platform-$arch"
mkdir -p "$target_dir"

echo "==> Building kestrel-lsp (release)"
( cd "$repo_root" && cargo build --release -p kestrel-lsp )

src="$repo_root/target/release/$exe_name"
dst="$target_dir/$exe_name"
echo "==> Staging $src -> $dst"
cp "$src" "$dst"
chmod +x "$dst"

echo "==> Done. Bundled binary: $dst"
