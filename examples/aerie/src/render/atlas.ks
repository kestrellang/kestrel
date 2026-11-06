module Aerie

import wgpu.(Device, Texture)
import images.(Image, Rgba8)

/// An atlas image that is a column of square tiles, loaded as a mipmapped
/// 2D array texture with one layer per tile, so tiles never bleed into
/// each other at a distance. Used for the block atlas (`assets/blocks.png`,
/// 16x16) and the interface atlas (`assets/ui.png`).
struct Atlas {
    /// `path`'s tiles, each `tileSize` pixels square. The atlases ship
    /// with the game, so a missing one is a broken install.
    static func load(device: Device, path: String, tileSize tileSize: Int64) -> Texture {
        let image = match Image[Rgba8].load(path: path) {
            .Ok(i) => i,
            .Err(e) => fatalError("can't load the atlas \(path): \(e.message)")
        };
        let layers = image.height / tileSize;
        let texture = device.createTexture(
            width: tileSize,
            height: tileSize,
            format: .RGBA8UnormSrgb,
            usage: [.TextureBinding, .CopyDst, .RenderAttachment],
            layers: layers,
            mipLevels: Texture.fullMipChain(width: tileSize, height: tileSize),
            label: path
        );
        // Tiles are stacked top to bottom, so each is one contiguous run
        // of the image's bytes.
        let tileBytes = tileSize * tileSize * 4;
        for layer in 0..<layers {
            let start = layer * tileBytes;
            device.write(texture: texture, data: image.bytes(start..<(start + tileBytes)).toArray(), layer: layer);
        }
        device.generateMipmaps(texture);
        texture
    }
}
