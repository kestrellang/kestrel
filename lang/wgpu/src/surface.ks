// Window surfaces: what a window shows, drawn into one frame at a time.

module wgpu

import wgpu.raw

/// The platform window a surface draws into, from a windowing library such
/// as `sdl` (its `SDLWindow.nativeWindow`).
public enum SurfaceSource {
    /// An X11 window: the `Display*` and the `Window` id.
    case Xlib(RawPointer, UInt64)
    /// A Wayland surface: the `wl_display*` and the `wl_surface*`.
    case Wayland(RawPointer, RawPointer)
    /// A macOS or iOS `CAMetalLayer*`.
    case MetalLayer(RawPointer)
    /// A Windows window: the `HINSTANCE` and the `HWND`.
    case Windows(RawPointer, RawPointer)
}

/// How presented frames line up with the display's refresh.
public enum PresentMode {
    /// Wait for vertical blank (vsync); always supported.
    case Fifo
    /// Vsync, but show a late frame at once.
    case FifoRelaxed
    /// No waiting; may tear.
    case Immediate
    /// Vsync, replacing a queued frame with a newer one.
    case Mailbox
}

/// A frame of a surface to draw into, from `Surface.currentTexture()`.
public struct SurfaceTexture: not Copyable {
    /// The frame's texture; draw into `texture.createView()`.
    public var texture: Texture
    /// Whether the surface still works but should be configured again (for
    /// example after a resize) to present efficiently.
    public var suboptimal: Bool
}

/// What a window shows.
public struct Surface: not Copyable {
    var handle: raw.WGPUSurface

    /// The format the surface prefers on `adapter`: the first it reports.
    public func preferredFormat(adapter: Adapter) -> TextureFormat {
        var caps = raw.WGPUSurfaceCapabilities(
            nextInChain: RawPointer.nullPointer(),
            usages: 0,
            formatCount: 0,
            formats: Pointer.nullPointer(),
            presentModeCount: 0,
            presentModes: Pointer.nullPointer(),
            alphaModeCount: 0,
            alphaModes: Pointer.nullPointer()
        );
        unsafe raw.wgpuSurfaceGetCapabilities(self.handle, adapter.handle, caps);
        let format = if caps.formatCount == 0 {
            TextureFormat.BGRA8Unorm
        } else {
            TextureFormat.fromC(unsafe caps.formats.read())
        };
        unsafe raw.wgpuSurfaceCapabilitiesFreeMembers(caps);
        format
    }

    /// Set the surface up to show `width` by `height` frames of `format`
    /// from `device`. Call again when the window resizes.
    public func configure(device device: Device, format format: TextureFormat, width width: Int64, height height: Int64, presentMode presentMode: PresentMode = .Fifo) {
        let config = raw.WGPUSurfaceConfiguration(
            nextInChain: Pointer.nullPointer(),
            device: device.handle,
            format: format.cValue,
            usage: raw.WGPUTextureUsage_RenderAttachment,
            width: UInt32(from: width),
            height: UInt32(from: height),
            viewFormatCount: 0,
            viewFormats: Pointer.nullPointer(),
            alphaMode: raw.WGPUCompositeAlphaMode_Auto,
            presentMode: match presentMode {
                .Fifo => raw.WGPUPresentMode_Fifo,
                .FifoRelaxed => raw.WGPUPresentMode_FifoRelaxed,
                .Immediate => raw.WGPUPresentMode_Immediate,
                .Mailbox => raw.WGPUPresentMode_Mailbox
            }
        );
        unsafe raw.wgpuSurfaceConfigure(self.handle, config);
    }

    /// The next frame to draw. Throws `.SurfaceOutdated` when the surface
    /// must be configured again (say, after a resize), `.SurfaceTimeout`
    /// when no frame came in time, and `.SurfaceLost` when it is gone.
    public func currentTexture() -> SurfaceTexture throws WgpuError {
        var frame = raw.WGPUSurfaceTexture(nextInChain: RawPointer.nullPointer(), texture: RawPointer.nullPointer(), status: 0);
        unsafe raw.wgpuSurfaceGetCurrentTexture(self.handle, frame);
        let status = frame.status;
        if status == raw.WGPUSurfaceGetCurrentTextureStatus_SuccessOptimal
            or status == raw.WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal
        {
            return .Ok(SurfaceTexture(
                texture: Texture(handle: frame.texture),
                suboptimal: status == raw.WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal
            ));
        }
        // A failed acquire may still hand back a texture to release.
        if not frame.texture.isNull {
            unsafe raw.wgpuTextureRelease(frame.texture);
        }
        if status == raw.WGPUSurfaceGetCurrentTextureStatus_Timeout {
            throw WgpuError.SurfaceTimeout("timed out waiting for a frame");
        }
        if status == raw.WGPUSurfaceGetCurrentTextureStatus_Outdated {
            throw WgpuError.SurfaceOutdated("the surface changed; configure it again");
        }
        throw WgpuError.SurfaceLost("the surface was lost")
    }

    /// Show the frame drawn since `currentTexture()`.
    public func present() {
        unsafe raw.wgpuSurfacePresent(self.handle);
    }

    deinit {
        unsafe raw.wgpuSurfaceRelease(self.handle);
    }
}

extend Instance {
    /// A surface drawing into the window `source`, which must outlive it.
    public func createSurface(source: SurfaceSource, label label: String = "") -> Surface {
        var xlib = raw.WGPUSurfaceSourceXlibWindow(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_SurfaceSourceXlibWindow),
            display: RawPointer.nullPointer(),
            window: 0
        );
        var wayland = raw.WGPUSurfaceSourceWaylandSurface(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_SurfaceSourceWaylandSurface),
            display: RawPointer.nullPointer(),
            surface: RawPointer.nullPointer()
        );
        var metal = raw.WGPUSurfaceSourceMetalLayer(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_SurfaceSourceMetalLayer),
            layer: RawPointer.nullPointer()
        );
        var windows = raw.WGPUSurfaceSourceWindowsHWND(
            chain: raw.WGPUChainedStruct(next: Pointer.nullPointer(), sType: raw.WGPUSType_SurfaceSourceWindowsHWND),
            hinstance: RawPointer.nullPointer(),
            hwnd: RawPointer.nullPointer()
        );
        let chain: Pointer[raw.WGPUChainedStruct] = match source {
            .Xlib(display, window) => {
                xlib.display = display;
                xlib.window = window;
                Pointer(to: xlib).cast[raw.WGPUChainedStruct]()
            },
            .Wayland(display, surface) => {
                wayland.display = display;
                wayland.surface = surface;
                Pointer(to: wayland).cast[raw.WGPUChainedStruct]()
            },
            .MetalLayer(layer) => {
                metal.layer = layer;
                Pointer(to: metal).cast[raw.WGPUChainedStruct]()
            },
            .Windows(hinstance, hwnd) => {
                windows.hinstance = hinstance;
                windows.hwnd = hwnd;
                Pointer(to: windows).cast[raw.WGPUChainedStruct]()
            }
        };
        let descriptor = raw.WGPUSurfaceDescriptor(nextInChain: chain, label: view(label));
        Surface(handle: unsafe raw.wgpuInstanceCreateSurface(self.handle, descriptor))
    }
}
