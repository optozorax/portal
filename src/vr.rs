//! Rendering this app onto a virtual 3D monitor in a VR headset.
//!
//! This is the *client* half of a two-process split. Everything about the
//! headset -- the OpenXR session, the virtual screen and where it hangs -- lives
//! in the separate `portal-compositor` process.
//! This module's entire job is to draw the app twice, once per eye, into a
//! texture the compositor allocated, and say when it is done.
//!
//! The app therefore has no OpenXR dependency, no headset knowledge, and no VR
//! code path beyond "render the stereo pair you already know how to render".
//! It still owns a real OpenGL context, but the VR build creates that window
//! hidden. The compositor is the visible preview window and forwards its input
//! into miniquad's normal event path.
//!
//! Why a separate process
//! ----------------------
//! The previous version did the whole thing in here, which meant the raytrace
//! ran between `xrBeginFrame` and `xrEndFrame`. A slow scene therefore meant
//! missed headset frames and a juddering virtual screen -- the scene's
//! complexity directly degraded the comfort of the display.
//!
//! Now the compositor submits the most recent *complete* frame from this app
//! every headset frame, whether or not a new one arrived. The raytracer can run
//! at 12 fps and the virtual screen stays rock steady at 90. That decoupling is
//! the reason for the split.
//!
//! It also means the compositor can drive Meta's PC runtime directly over
//! D3D11, which does not support OpenGL at all -- so a Quest no longer has to
//! be routed through SteamVR.
//!
//! How a frame works
//! -----------------
//! [`Client::frame`] is the whole per-frame interface. In headless VR it waits
//! on the named pipe when the compositor has not asked for a frame yet; this is
//! an event-driven wait, not a timer-based render loop. In order:
//!
//! 1. Drain the pipe. If no frame has been requested, wait for the next server
//!    message. Only the newest request is kept; older ones are stale by
//!    definition.
//! 2. Take the requested slot's keyed mutex, which is how ownership of the
//!    shared texture passes between the two processes.
//! 3. Run the caller's `draw_eye` closure twice, once per eye, each time
//!    leaving that eye's view of the window in the backbuffer, and blit the
//!    backbuffer into that eye's half of the shared texture.
//! 4. Hand the texture back and report the frame.
//!
//! Drawing the window twice is forced, not sloppiness: `egui_miniquad::Painter`
//! hardcodes `begin_default_pass`, so egui can only ever paint into the
//! backbuffer, and `draw` consumes its shapes so it can only do it once per
//! run. Since egui has to end up *on top of* each eye's canvas rather than
//! under it, two runs are the only way to get two correctly layered images.
//! egui is immediate mode and does not mind; the app logic threaded through the
//! same closure does, which is what the caller's `apply_input` guard is for.
//! The raytrace, the expensive part, still happens at most once.
//!
//! Getting OpenGL pixels into a D3D11 texture
//! ------------------------------------------
//! The compositor is D3D11 and this app is OpenGL, so something has to bridge
//! them. That something is `WGL_NV_DX_interop2`, which is not an exotic choice:
//! it is the same mechanism SteamVR and Oculus use internally to offer OpenGL
//! swapchains at all. Using it directly means the failures are ours to see
//! rather than a black box's.
//!
//! There is one wrinkle, and [`Staging`] exists because of it. The compositor's
//! shared textures carry keyed mutexes, and registering a keyed-mutex resource
//! with the interop extension is not reliably supported. So GL never touches
//! them. Instead this module keeps one plain, private D3D11 texture, registers
//! *that* with the extension, blits into it with GL, and then copies the two
//! eye rectangles across with `CopySubresourceRegion` under the keyed mutex.
//! The copy is GPU-local and only covers the pixels that changed.
//!
//! The staging texture is deliberately **not** sRGB even though the
//! compositor's is. Drivers routinely refuse to register an sRGB texture with
//! the interop extension, and a plain `UNORM` one is the more honest
//! description anyway: the blit copies bytes out of the backbuffer with
//! `GL_FRAMEBUFFER_SRGB` off, so what lands there is already-encoded sRGB data
//! being moved verbatim. Copying it into the compositor's sRGB texture is legal
//! because the two formats share a typeless parent, and correct because the
//! bytes were sRGB all along.

use macroquad::miniquad::gl;
use macroquad::prelude::*;
use portal_vr_protocol as proto;
use std::ffi::c_void;
use std::time::Instant;
use windows::core::Interface;
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, HWND, RECT};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11Device1, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_CPU_ACCESS_WRITE,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_WRITE, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1, IDXGIKeyedMutex, DXGI_ERROR_NOT_FOUND,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER,
};

pub use portal_vr_protocol::Eye;

/// `GL_FRAMEBUFFER_SRGB`, which miniquad's GL bindings do not declare.
const GL_FRAMEBUFFER_SRGB: gl::GLenum = 0x8DB9;

/// `WGL_ACCESS_READ_WRITE_NV`.
///
/// The GL side only writes this resource, but READ_WRITE is the access mode
/// used by the established D3D/GL interop implementations (including Spout)
/// and is more widely tolerated by drivers when the resource is later read by
/// D3D11. WRITE_DISCARD is legal in the extension, but some drivers accept the
/// registration and then do not expose the written image reliably.
const WGL_ACCESS_READ_WRITE_NV: gl::GLenum = 0x0001;

/// `WAIT_TIMEOUT` as `AcquireSync` returns it. See [`acquire`].
const WAIT_TIMEOUT_HRESULT: u32 = 0x0000_0102;
/// `WAIT_ABANDONED` as `AcquireSync` returns it.
const WAIT_ABANDONED_HRESULT: u32 = 0x0000_0080;

/// How long to wait for the compositor to let go of a slot.
///
/// It should already have: the compositor never asks for a slot it is holding.
/// A short wait rather than none covers the moment right after a handover, and
/// a short wait rather than a long one means a confused compositor cannot stall
/// this app's window.
const ACQUIRE_TIMEOUT_MS: u32 = 4;

/// The app's connection to the compositor.
///
/// [`Client::frame`] is the whole per-frame interface; see the module docs.
pub struct Client {
    pipe: Pipe,
    /// Held for its lifetime rather than used directly: everything below was
    /// created from it, and dropping it would take them with it.
    #[allow(dead_code)]
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    layout: proto::TextureLayout,
    /// The compositor's textures, opened here. Never touched by OpenGL.
    slots: Vec<Slot>,
    transport: Transport,
    /// Framebuffer object the staging texture is attached to for the blit. The
    /// read side is the backbuffer, which is framebuffer zero.
    draw_fbo: gl::GLuint,
    /// Local mirror of the compositor's screen settings. See [`ScreenSettings`].
    pub screen: ScreenSettings,
    /// Why the connection ended, if it has. Kept rather than dropped on the
    /// floor, because someone wearing a headset cannot read stderr and the
    /// settings panel is the only place left to say it.
    stopped: Option<String>,
    system_name: String,
    /// Parallax shift between the eyes, in pixels of this window.
    ///
    /// Lives here rather than in the compositor because it is a property of how
    /// *this app* renders stereo, not of the screen it lands on. The eye
    /// cameras are offset laterally with identical frusta, which puts infinity
    /// exactly at the screen plane and makes everything nearer pop out in front
    /// of it -- comfortable for a while, tiring for long. Positive values push
    /// the scene back behind the screen, which is the classic fix.
    pub convergence_px: f32,
    /// Whether the compositor should flip the client image inside its window.
    ///
    /// The transport keeps the raw OpenGL row order; the compositor applies
    /// this only while sampling the client rectangle, so its desktop and
    /// border remain upright.
    pub flip_y: bool,
    /// Frames handed over, for the settings panel.
    frames: u64,
    /// A resize requested by the compositor but not visible in miniquad's
    /// framebuffer yet. `set_window_size` is delivered through miniquad's
    /// native request queue, so the current draw must finish before
    /// `screen_size()` changes.
    pending_resize: Option<(u32, u32)>,
}

/// Where the virtual screen hangs, as far as this app knows.
///
/// The compositor is the authority on all of it; this is a write-through mirror
/// so the settings panel has something to move. That is why the values start at
/// the compositor's documented defaults rather than being read back: there is
/// no message to ask, because nothing needs one -- whoever moves a slider is
/// looking at the screen while they do it.
#[derive(Clone, Copy, Debug)]
pub struct ScreenSettings {
    /// Metres in front of you.
    pub distance: f32,
    /// Metres above the anchor.
    pub height: f32,
    /// Metres to the right of the anchor.
    pub lateral: f32,
    /// Width of the screen in metres; height follows its aspect ratio.
    pub width_meters: f32,
    /// Rotation about Y, radians.
    pub yaw: f32,
    /// Rotation about X, radians.
    pub pitch: f32,
    /// Reserved size multiplier for a future compositor-drawn VR cursor.
    pub cursor_scale: f32,
    /// Compositor MSAA setting for the desktop border, frame and cursor.
    pub msaa_samples: u32,
}

impl Default for ScreenSettings {
    fn default() -> Self {
        Self {
            distance: 1.6,
            height: 0.0,
            lateral: 0.0,
            width_meters: 1.6,
            yaw: 0.0,
            pitch: 0.0,
            cursor_scale: 2.0,
            msaa_samples: 4,
        }
    }
}

/// One of the compositor's shared textures, opened in this process.
struct Slot {
    texture: ID3D11Texture2D,
    mutex: IDXGIKeyedMutex,
}

/// How this app's pixels get from OpenGL into a D3D11 texture.
///
/// Two ways. The fast one is the only normal path; the slow one remains
/// available when explicitly requested for diagnostics.
enum Transport {
    /// `WGL_NV_DX_interop2`: the GPU writes once and nothing is copied over the
    /// bus. What this should be doing.
    Interop(Staging),
    /// `glReadPixels` into system memory and back up through a D3D11 staging
    /// texture. Roughly 30 MB a frame across the bus in each direction and it
    /// stalls the GL pipeline, so it is much slower -- but it needs no
    /// extension, no driver cooperation and no shared memory at all, so it
    /// cannot fail for any of the reasons the fast path can. It is retained
    /// only as an explicit diagnostic mode, not as a production fallback.
    Readback(Readback),
}

impl Transport {
    fn new(device: &ID3D11Device, layout: &proto::TextureLayout) -> Result<Self, String> {
        if force_readback() {
            return Ok(Self::Readback(Readback::new(device, layout)?));
        }
        match Staging::new(device, layout) {
            Ok(staging) => Ok(Self::Interop(staging)),
            Err(err) => Err(format!(
                "WGL_NV_DX_interop2 setup failed: {err}. CPU readback is disabled by default; \
                 set PORTAL_VR_READBACK=1 only for diagnostics."
            )),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Interop(_) => "WGL_NV_DX_interop2",
            Self::Readback(_) => "glReadPixels (explicit diagnostic)",
        }
    }
}

/// The slow path: read the backbuffer into system memory and write it into a
/// D3D11 staging texture the CPU can touch.
struct Readback {
    /// `USAGE_STAGING` with CPU write access, so it can be mapped and then
    /// copied into the compositor's texture on the GPU.
    texture: ID3D11Texture2D,
    /// Scratch for `glReadPixels`, grown as needed and then reused.
    pixels: Vec<u8>,
}

/// A private D3D11 texture that OpenGL is allowed to draw into.
///
/// See the module docs: the compositor's own textures carry keyed mutexes and
/// registering one of those with `WGL_NV_DX_interop2` is not reliably
/// supported, so GL writes here and D3D copies across.
struct Staging {
    texture: ID3D11Texture2D,
    /// The GL texture name the interop extension ties to `texture`.
    gl_name: gl::GLuint,
    /// What `wglDXRegisterObjectNV` returned, needed to lock and unlock.
    object: HANDLE,
    /// What `wglDXOpenDeviceNV` returned.
    interop_device: HANDLE,
    wgl: Wgl,
}

impl Client {
    /// Connect to the compositor and set up everything a frame needs.
    ///
    /// Must be called with the GL context current, i.e. from inside the
    /// macroquad main loop, after the window exists.
    pub fn open() -> Result<Self, String> {
        let mut pipe = Pipe::connect()?;

        pipe.send(&proto::ClientMessage::Hello {
            protocol_version: proto::PROTOCOL_VERSION,
            app_name: "Portal Explorer".to_owned(),
            backend: if force_readback() {
                proto::ClientBackend::CpuReadback
            } else {
                proto::ClientBackend::OpenGlNvDxInterop
            },
        })?;

        let welcome = match pipe.recv_blocking()? {
            proto::ServerMessage::Welcome(welcome) => welcome,
            proto::ServerMessage::Shutdown { reason } => {
                return Err(format!("the compositor refused the connection: {reason}"))
            }
            other => return Err(format!("expected a welcome, got {other:?}")),
        };
        // The compositor's adapter, which it got from OpenXR. Opening a shared
        // texture from a device on a different adapter either fails outright or
        // is unusably slow, and the failure looks like a black screen.
        let (device, context) = create_device(welcome.adapter_luid)?;
        let device1: ID3D11Device1 = device
            .cast()
            .map_err(|err| format!("D3D11.1 is required to open shared NT handles: {err}"))?;

        let slots = welcome
            .slots
            .iter()
            .enumerate()
            .map(|(index, slot)| open_slot(&device1, slot, index))
            .collect::<Result<Vec<_>, String>>()?;
        let transport = Transport::new(&device, &welcome.layout)?;
        log(format!(
            "connected to {} using {} at {}x{} per eye",
            welcome.system_name,
            transport.name(),
            welcome.layout.eye_width,
            welcome.layout.eye_height
        ));

        let mut draw_fbo = 0;
        unsafe { gl::glGenFramebuffers(1, &mut draw_fbo) };

        Ok(Self {
            pipe,
            device,
            context,
            layout: welcome.layout,
            slots,
            transport,
            draw_fbo,
            screen: ScreenSettings::default(),
            stopped: None,
            system_name: welcome.system_name,
            convergence_px: 120.0,
            flip_y: true,
            frames: 0,
            pending_resize: None,
        })
    }

    /// Whether frames are still going to the compositor. Goes false for good
    /// once something fails hard enough to give up on.
    pub fn is_active(&self) -> bool {
        self.stopped.is_none()
    }

    /// Draw one frame for the compositor.
    ///
    /// `draw_eye` is called once per eye, in [`Eye::BOTH`] order, and must
    /// leave that eye's view of the whole window in the backbuffer. The client
    /// reference lets the caller draw client-owned UI while the client is
    /// temporarily rendering, without taking a second mutable borrow around
    /// [`Client::frame`].
    ///
    /// Returns whether a frame was handed over. In the headless VR client this
    /// waits efficiently for the compositor's next request instead of polling
    /// with a millisecond sleep. The caller has no visible window to keep
    /// refreshing, and waiting on the pipe avoids Windows' coarse sleep
    /// quantization that otherwise turns a 120 Hz client into a 60 Hz client.
    pub fn frame(&mut self, draw_eye: impl FnMut(Eye, &mut Client)) -> bool {
        if self.stopped.is_some() {
            return false;
        }
        match self.try_frame(draw_eye) {
            Ok(rendered) => rendered,
            Err(err) => {
                log(format!("stopped: {err}"));
                self.stopped = Some(err);
                false
            }
        }
    }

    fn try_frame(&mut self, mut draw_eye: impl FnMut(Eye, &mut Client)) -> Result<bool, String> {
        // Only the newest request matters. An older one describes a moment the
        // compositor has already moved past, and answering it would put a stale
        // image on screen.
        let mut request = None;
        while let Some(message) = self.pipe.try_recv()? {
            self.handle_server_message(message, &mut request)?;
        }
        let request = loop {
            if let Some(request) = request.take() {
                break request;
            }

            // A Windows sleep of one millisecond is normally rounded to the
            // system timer quantum (often 15.6 ms). That was the hidden 60 Hz
            // cap: the client simply did not inspect the pipe in time for
            // every 120 Hz compositor request. ReadFile blocks in the kernel
            // until the next complete protocol message instead.
            let message = self.pipe.recv_blocking()?;
            self.handle_server_message(message, &mut request)?;
        };
        if !request.should_render {
            // A skipped request still has to acknowledge the compositor's
            // single in-flight request. Otherwise one OpenXR frame that the
            // runtime does not intend to display permanently wedges the
            // pipeline: the compositor waits for FrameReady and the client
            // quite correctly never renders one.
            self.pipe.send(&proto::ClientMessage::FrameSkipped {
                frame_id: request.frame_id,
                slot: request.slot,
                slot_released: false,
            })?;
            return Ok(false);
        }

        let index = request.slot as usize;
        let slot = self.slots.get(index).ok_or_else(|| {
            format!("the compositor asked for slot {index}, which does not exist")
        })?;

        let acquired = acquire(&slot.mutex, proto::KEY_CLIENT, ACQUIRE_TIMEOUT_MS)?;
        if !acquired {
            // The compositor still has it. Not an error and not worth waiting
            // for; it will ask again.
            return Ok(false);
        }
        let frame_started = Instant::now();

        // The compositor's preview is the client's emulated ordinary window.
        // Ask the hidden native window to follow it, but do not make a resize
        // acknowledgement a prerequisite for rendering. Native window
        // resizing is asynchronous and a headless surface may not report the
        // new framebuffer immediately; rendering the current size is still
        // useful and avoids turning a transient resize into a permanent black
        // screen. `render_into` reports the actual rectangle it filled.
        let _ = self.apply_requested_size(request.client_px)?;

        let result = self.render_into(index, &request, &mut draw_eye);

        // The mutex has to go back whatever happened, or the compositor waits
        // for a slot that is never coming and the virtual screen freezes.
        let released = release(&self.slots[index].mutex, proto::KEY_SERVER);
        released?;

        let content = result?;

        // What was actually filled, not what was asked for. Normally this is
        // exactly the emulated preview-window size; reporting the measured
        // rectangle keeps a transient DPI or resize mismatch from exposing
        // stale pixels in the shared texture.
        let frame_time_us = frame_started.elapsed().as_micros().min(u32::MAX as u128) as u32;
        let send_result = self.pipe.send(&proto::ClientMessage::FrameReady {
            frame_id: request.frame_id,
            slot: request.slot,
            content,
            frame_time_us,
        });
        send_result?;
        self.frames += 1;
        Ok(true)
    }

    fn handle_server_message(
        &mut self,
        message: proto::ServerMessage,
        request: &mut Option<proto::FrameRequest>,
    ) -> Result<(), String> {
        match message {
            proto::ServerMessage::Frame(frame) => {
                *request = Some(frame);
                Ok(())
            }
            proto::ServerMessage::Shutdown { reason } => {
                Err(format!("the compositor shut us down: {reason}"))
            }
            proto::ServerMessage::Input(events) => {
                for event in events {
                    self.inject_input(event);
                }
                Ok(())
            }
            proto::ServerMessage::Welcome(_) => Ok(()),
        }
    }

    fn apply_requested_size(&mut self, requested: [u32; 2]) -> Result<bool, String> {
        let target = (requested[0].max(1), requested[1].max(1));
        let (width, height) = macroquad::miniquad::window::screen_size();
        let actual = (width.round() as u32, height.round() as u32);
        if actual == target {
            self.pending_resize = None;
            return Ok(true);
        }

        if self.pending_resize != Some(target) {
            resize_hidden_window(target)?;
            self.pending_resize = Some(target);
        }
        Ok(false)
    }

    /// Convert compositor-window coordinates from the shared virtual-screen
    /// space into this client's actual framebuffer space, then feed them into
    /// miniquad's normal event path.
    fn inject_input(&self, event: proto::InputEvent) {
        let (width, height) = macroquad::miniquad::window::screen_size();
        let scale_x = width / self.layout.eye_width.max(1) as f32;
        let scale_y = height / self.layout.eye_height.max(1) as f32;
        let mods = |bits: u8| macroquad::miniquad::KeyMods {
            shift: bits & proto::MOD_SHIFT != 0,
            ctrl: bits & proto::MOD_CONTROL != 0,
            alt: bits & proto::MOD_ALT != 0,
            logo: bits & proto::MOD_LOGO != 0,
        };
        let event = match event {
            proto::InputEvent::MouseMotion { x, y } => {
                macroquad::miniquad::InjectedEvent::MouseMotion {
                    x: x * scale_x,
                    y: y * scale_y,
                }
            }
            proto::InputEvent::MouseDown { x, y, button } => {
                macroquad::miniquad::InjectedEvent::MouseDown {
                    x: x * scale_x,
                    y: y * scale_y,
                    button: match button {
                        proto::MouseButton::Left => macroquad::miniquad::MouseButton::Left,
                        proto::MouseButton::Right => macroquad::miniquad::MouseButton::Right,
                        proto::MouseButton::Middle => macroquad::miniquad::MouseButton::Middle,
                    },
                }
            }
            proto::InputEvent::MouseUp { x, y, button } => {
                macroquad::miniquad::InjectedEvent::MouseUp {
                    x: x * scale_x,
                    y: y * scale_y,
                    button: match button {
                        proto::MouseButton::Left => macroquad::miniquad::MouseButton::Left,
                        proto::MouseButton::Right => macroquad::miniquad::MouseButton::Right,
                        proto::MouseButton::Middle => macroquad::miniquad::MouseButton::Middle,
                    },
                }
            }
            proto::InputEvent::MouseWheel { dx, dy } => {
                macroquad::miniquad::InjectedEvent::MouseWheel { dx, dy }
            }
            proto::InputEvent::KeyDown {
                keycode,
                modifiers,
                repeat,
            } => macroquad::miniquad::InjectedEvent::KeyDown {
                scancode: keycode,
                mods: mods(modifiers),
                repeat,
            },
            proto::InputEvent::KeyUp { keycode, modifiers } => {
                macroquad::miniquad::InjectedEvent::KeyUp {
                    scancode: keycode,
                    mods: mods(modifiers),
                }
            }
            proto::InputEvent::Char {
                character,
                modifiers,
            } => macroquad::miniquad::InjectedEvent::Char {
                character,
                mods: mods(modifiers),
                repeat: false,
            },
            proto::InputEvent::Focus(focused) => macroquad::miniquad::InjectedEvent::Focus(focused),
        };
        macroquad::miniquad::window::inject_event(event);
    }

    /// Draw both eyes and get them into the shared texture, returning the
    /// rectangles actually filled. The keyed mutex is held for the whole of it.
    fn render_into(
        &mut self,
        index: usize,
        request: &proto::FrameRequest,
        draw_eye: &mut impl FnMut(Eye, &mut Client),
    ) -> Result<[proto::Rect; 2], String> {
        // What this window can actually supply: its current framebuffer size,
        // capped only at the shared texture's per-eye capacity. It is allowed
        // to differ from the compositor's requested size while a native resize
        // is pending; the compositor scales the reported rectangle into the
        // emulated window.
        let (width, height) = macroquad::miniquad::window::screen_size();
        let content = Eye::BOTH.map(|eye| {
            let asked = request.content[eye.index()];
            // `asked.x` is an absolute x-coordinate in the double-wide
            // shared texture. The framebuffer we render is local to one eye,
            // so the available width must be measured against the full
            // double-wide texture, not against `eye_width`. Otherwise the
            // right eye (whose x is `eye_width`) is reported as width zero and
            // the compositor rejects every otherwise valid stereo frame.
            let bounds_width = self.layout.width.saturating_sub(asked.x);
            let bounds_height = self.layout.eye_height.saturating_sub(asked.y);
            proto::Rect::new(
                asked.x,
                asked.y,
                (width.max(1.0) as u32).min(bounds_width),
                (height.max(1.0) as u32).min(bounds_height),
            )
        });

        // Copied out so the transport can be borrowed mutably below without
        // taking the rest of `self` with it. Both are cheap: a COM pointer and
        // an integer.
        let context = self.context.clone();
        let draw_fbo = self.draw_fbo;
        let test_pattern = interop_test_pattern();

        // GL may only touch the interop texture between lock and unlock. Keep
        // the lock across both eyes, but do not keep a mutable borrow of the
        // transport across `draw_eye`: the callback also needs `&mut Client`
        // so it can render the Virtual screen settings window.
        let interop_gl_name = match &mut self.transport {
            Transport::Interop(staging) => {
                staging.lock()?;
                Some(staging.gl_name)
            }
            Transport::Readback(_) => None,
        };

        let render_result = (|| -> Result<(), String> {
            for eye in Eye::BOTH {
                draw_eye(eye, self);
                flush_gl();
                if let Some(gl_name) = interop_gl_name {
                    copy_backbuffer(draw_fbo, gl_name, content[eye.index()], test_pattern)?;
                } else if let Transport::Readback(readback) = &mut self.transport {
                    // Keep the shared texture in the same raw OpenGL row
                    // order as the GPU interop path. The compositor flips
                    // only this client rectangle when it draws the desktop;
                    // flipping here too would undo that there.
                    readback.capture(&context, content[eye.index()], false)?;
                }
            }
            Ok(())
        })();

        // Always unlock, including when framebuffer validation or the blit
        // itself fails. Leaving the interop object locked makes even a clean
        // shutdown impossible.
        let unlock_result = if interop_gl_name.is_some() {
            match &mut self.transport {
                Transport::Interop(staging) => staging.unlock(),
                Transport::Readback(_) => Ok(()),
            }
        } else {
            Ok(())
        };
        render_result?;
        unlock_result?;

        let source = self.transport_texture();
        for eye in Eye::BOTH {
            self.copy_to_shared(index, &source, content[eye.index()]);
        }
        // Submitted before the mutex is released, or the compositor can start
        // reading a texture whose commands are still queued here.
        unsafe { self.context.Flush() };
        Ok(content)
    }

    /// The texture the current transport left this frame's pixels in.
    fn transport_texture(&self) -> ID3D11Texture2D {
        match &self.transport {
            Transport::Interop(staging) => staging.texture.clone(),
            Transport::Readback(readback) => readback.texture.clone(),
        }
    }

    /// Copy one eye's rectangle from the transport's texture into the shared
    /// one.
    fn copy_to_shared(&self, index: usize, source: &ID3D11Texture2D, rect: proto::Rect) {
        let box_ = D3D11_BOX {
            left: rect.x,
            top: rect.y,
            front: 0,
            right: rect.x + rect.width,
            bottom: rect.y + rect.height,
            back: 1,
        };
        unsafe {
            self.context.CopySubresourceRegion(
                &self.slots[index].texture,
                0,
                rect.x,
                rect.y,
                0,
                source,
                0,
                Some(&box_),
            );
        }
    }

    /// Settings for the virtual screen.
    ///
    /// Most of them belong to the compositor, so this sends them over rather
    /// than applying them: the compositor is what decides where the screen
    /// hangs. Convergence is the exception -- that is about how this app draws
    /// stereo, so it stays here.
    pub fn ui(&mut self, ui: &mut egui::Ui, send_controls: bool) {
        if let Some(stopped) = &self.stopped {
            ui.colored_label(egui::Color32::RED, format!("VR stopped: {stopped}"));
            ui.label("The window is being drawn the ordinary way again. Restart to try again.");
            ui.separator();
        }

        ui.label(format!("Headset: {}", self.system_name));
        ui.label(format!("Transport: {}", self.transport.name()));
        ui.label(format!(
            "Virtual screen: {}x{} per eye, {} frames sent",
            self.layout.eye_width, self.layout.eye_height, self.frames
        ));

        ui.separator();
        let mut control = proto::ScreenControl::default();
        let mut changed = false;

        if ui
            .button("⊕ Place screen in front of me")
            .on_hover_text("Moves the screen to where you are looking, keeping the offsets below")
            .clicked()
        {
            control.place_in_front_of_me = true;
            changed = true;
        }

        // Local mirrors of the compositor's values. It owns them, so these are
        // only ever write-through -- which is why they start at the compositor's
        // documented defaults rather than being read back.
        if slider(ui, &mut self.screen.width_meters, 0.2..=3.0, "width, m") {
            control.width_meters = Some(self.screen.width_meters);
            changed = true;
        }
        if slider(ui, &mut self.screen.distance, 0.2..=3.0, "distance, m") {
            control.distance = Some(self.screen.distance);
            changed = true;
        }
        if slider(ui, &mut self.screen.height, -2.0..=2.0, "height, m") {
            control.height = Some(self.screen.height);
            changed = true;
        }
        if slider(ui, &mut self.screen.lateral, -2.0..=2.0, "sideways, m") {
            control.lateral = Some(self.screen.lateral);
            changed = true;
        }
        if slider(ui, &mut self.screen.cursor_scale, 0.5..=8.0, "cursor size") {
            control.cursor_scale = Some(self.screen.cursor_scale);
            changed = true;
        }

        let mut yaw = self.screen.yaw.to_degrees();
        if slider(ui, &mut yaw, -180.0..=180.0, "yaw, °") {
            self.screen.yaw = yaw.to_radians();
            control.yaw = Some(self.screen.yaw);
            changed = true;
        }
        let mut pitch = self.screen.pitch.to_degrees();
        if slider(ui, &mut pitch, -90.0..=90.0, "pitch, °") {
            self.screen.pitch = pitch.to_radians();
            control.pitch = Some(self.screen.pitch);
            changed = true;
        }

        let mut msaa = self.screen.msaa_samples;
        if ui
            .add(
                egui::Slider::new(&mut msaa, 1..=8)
                    .text("antialiasing (MSAA)")
                    .step_by(1.0)
                    .clamping(egui::widgets::SliderClamping::Always),
            )
            .changed()
        {
            self.screen.msaa_samples = msaa;
            control.msaa_samples = Some(self.screen.msaa_samples);
            changed = true;
        }
        ui.label("MSAA affects the compositor's screen border, window frame and cursor.");

        if changed && send_controls {
            if let Err(err) = self
                .pipe
                .send(&proto::ClientMessage::ScreenControl(control))
            {
                log(format!("could not send screen settings: {err}"));
            }
        }

        ui.separator();
        ui.add(
            egui::Slider::new(&mut self.convergence_px, -120.0..=120.0)
                .text("convergence, screen px")
                .clamping(egui::widgets::SliderClamping::Always),
        );
        ui.label(
            "The eye cameras are parallel, which puts infinity level with the screen and makes \
             everything nearer float in front of it. Raise this to push the scene back behind \
             the screen instead. It slides each eye's canvas sideways by half of it, which is \
             why the canvas sits off-centre in the client preview while VR is on.",
        );
        ui.label("(3D strength itself is `Eye distance` under camera settings.)");

        ui.separator();
        ui.collapsing("If it looks wrong", |ui| {
            let flip_changed = ui
                .checkbox(&mut self.flip_y, "Screen is upside down")
                .changed();
            ui.label(
                "An OpenGL backbuffer is bottom-up while a D3D11 texture is top-down, so the \
                 image is flipped on its way across. If it comes out inverted anyway, untick \
                 this.",
            );
            ui.separator();
            ui.label(
                "Both eyes look identical: only the raytraced canvas is stereo in the first \
                 place -- the egui windows and the window frame are meant to look flat. If the \
                 canvas is flat too, raise `Eye distance` under camera settings.",
            );
            ui.separator();
            ui.label(
                "Everything looks low resolution: the headset resolves roughly 21 pixels per \
                 degree, so a screen this wide can only ever show about `width in degrees x 21` \
                 pixels across, however many the desktop actually has. Make the screen bigger \
                 and closer, and raise `Interface DPI`.",
            );

            // The compositor owns the actual flip because it is the process
            // that samples the shared D3D11 texture into OpenXR. Keep this
            // diagnostic checkbox useful by sending the setting over the same
            // control channel as the other virtual-screen settings.
            if flip_changed && send_controls {
                if let Err(err) =
                    self.pipe
                        .send(&proto::ClientMessage::ScreenControl(proto::ScreenControl {
                            flip_y: Some(self.flip_y),
                            ..Default::default()
                        }))
                {
                    log(format!("could not send flip setting: {err}"));
                }
            }
        });
    }
}

/// Resize the actual hidden miniquad window, preserving its non-client frame.
///
/// `miniquad::window::set_window_size` queues a native request. That request is
/// not reliable for this headless configuration: the hidden window can keep
/// reporting its old framebuffer forever, which makes every compositor frame
/// get skipped. The HWND is deliberately exposed by `miniquad-vr`, so apply
/// the resize synchronously and let miniquad observe it on its next dimension
/// update.
fn resize_hidden_window(target: (u32, u32)) -> Result<(), String> {
    let handle = macroquad::miniquad::window::native_window_handle()
        .ok_or_else(|| "miniquad has not created the hidden Portal window".to_owned())?;
    let hwnd = HWND(handle.get() as *mut c_void);
    let mut outer = RECT::default();
    let mut client = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer)
            .map_err(|err| format!("GetWindowRect for hidden Portal window failed: {err}"))?;
        GetClientRect(hwnd, &mut client)
            .map_err(|err| format!("GetClientRect for hidden Portal window failed: {err}"))?;
    }

    let outer_width = (outer.right - outer.left).max(0) as u32;
    let outer_height = (outer.bottom - outer.top).max(0) as u32;
    let client_width = (client.right - client.left).max(0) as u32;
    let client_height = (client.bottom - client.top).max(0) as u32;
    let frame_width = outer_width.saturating_sub(client_width);
    let frame_height = outer_height.saturating_sub(client_height);
    let outer_size = (
        target.0.saturating_add(frame_width),
        target.1.saturating_add(frame_height),
    );

    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            outer_size.0.min(i32::MAX as u32) as i32,
            outer_size.1.min(i32::MAX as u32) as i32,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    }
    .map_err(|err| format!("SetWindowPos for hidden Portal window failed: {err}"))
}

/// Copy the backbuffer into `rect` of the interop staging texture.
///
/// The normal path uses `glCopyTexSubImage2D`: it is a GPU copy from the
/// current read buffer directly into the registered D3D11 texture and avoids
/// the NVIDIA driver's problematic default-framebuffer-to-interop-FBO blit.
/// The FBO is retained only for the GPU test pattern.
fn copy_backbuffer(
    draw_fbo: gl::GLuint,
    gl_name: gl::GLuint,
    rect: proto::Rect,
    test_pattern: bool,
) -> Result<(), String> {
    let (width, height) = (rect.width as i32, rect.height as i32);
    if width <= 0 || height <= 0 {
        return Ok(());
    }

    unsafe {
        // Do not report an error left behind by macroquad or egui as an error
        // in this copy. The old in-process OpenXR path explicitly drained GL
        // errors for the same reason; this code must do that before it starts
        // diagnosing the interop operation.
        while gl::glGetError() != gl::GL_NO_ERROR {}
        gl::glBindFramebuffer(gl::GL_READ_FRAMEBUFFER, 0);
        gl::glReadBuffer(gl::GL_BACK);

        if test_pattern {
            // GPU-only diagnostic. Attach the registered texture as an FBO
            // target and clear it. If this solid color reaches the headset,
            // interop registration and the D3D11 copy are alive.
            gl::glBindFramebuffer(gl::GL_DRAW_FRAMEBUFFER, draw_fbo);
            gl::glFramebufferTexture2D(
                gl::GL_DRAW_FRAMEBUFFER,
                gl::GL_COLOR_ATTACHMENT0,
                gl::GL_TEXTURE_2D,
                gl_name,
                0,
            );
            let draw_buffer = gl::GL_COLOR_ATTACHMENT0;
            gl::glDrawBuffers(1, &draw_buffer);
            gl::glDisable(GL_FRAMEBUFFER_SRGB);
            gl::glDisable(gl::GL_SCISSOR_TEST);
            let status = gl::glCheckFramebufferStatus(gl::GL_DRAW_FRAMEBUFFER);
            if status != gl::GL_FRAMEBUFFER_COMPLETE {
                gl::glBindFramebuffer(gl::GL_FRAMEBUFFER, 0);
                return Err(format!(
                    "interop staging framebuffer is incomplete: status {status:#x}"
                ));
            }
            gl::glClearColor(1.0, 0.0, 1.0, 1.0);
            gl::glClear(gl::GL_COLOR_BUFFER_BIT);
            gl::glBindFramebuffer(gl::GL_FRAMEBUFFER, 0);
        } else {
            gl::glBindTexture(gl::GL_TEXTURE_2D, gl_name);
            gl::glCopyTexSubImage2D(
                gl::GL_TEXTURE_2D,
                0,
                rect.x as i32,
                rect.y as i32,
                0,
                0,
                width,
                height,
            );
            gl::glBindTexture(gl::GL_TEXTURE_2D, 0);
        }
        // WGL interop lock/unlock is the resource ownership boundary, but an
        // explicit flush makes the GL->D3D ordering unambiguous on drivers
        // that do not treat unlock as a sufficient command submission point.
        gl::glFlush();

        // Leave the binding as macroquad expects to find it.
        gl::glBindFramebuffer(gl::GL_FRAMEBUFFER, 0);

        let error = gl::glGetError();
        if error != gl::GL_NO_ERROR {
            return Err(format!("OpenGL interop copy failed with error {error:#x}"));
        }
    }
    Ok(())
}

/// macroquad batches its draw calls, and everything below goes straight through
/// GL, so its queue has to be emptied first.
fn flush_gl() {
    unsafe { macroquad::window::get_internal_gl() }.flush();
}

/// `GL_BGRA`, which miniquad's GL bindings do not declare. Matches the byte
/// order of the D3D11 `B8G8R8A8` textures, so no swizzling is needed.
const GL_BGRA: gl::GLenum = 0x80E1;

impl Readback {
    fn new(device: &ID3D11Device, layout: &proto::TextureLayout) -> Result<Self, String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: layout.width,
            Height: layout.height,
            MipLevels: 1,
            ArraySize: 1,
            // Match the compositor's resource exactly. Mapping an sRGB
            // texture still exposes the same four bytes; the format tag only
            // matters to shader conversion, which this staging texture never
            // uses.
            Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: 0,
        };
        let mut texture = None;
        unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture)) }
            .map_err(|err| format!("CreateTexture2D for the readback texture failed: {err}"))?;
        Ok(Self {
            texture: texture.ok_or("CreateTexture2D returned nothing")?,
            pixels: Vec::new(),
        })
    }

    /// Read the backbuffer and write it into the staging texture at `rect`.
    fn capture(
        &mut self,
        context: &ID3D11DeviceContext,
        rect: proto::Rect,
        flip_y: bool,
    ) -> Result<(), String> {
        let stride = rect.width as usize * 4;
        let needed = stride * rect.height as usize;
        if self.pixels.len() < needed {
            self.pixels.resize(needed, 0);
        }
        unsafe {
            gl::glBindFramebuffer(gl::GL_READ_FRAMEBUFFER, 0);
            gl::glReadBuffer(gl::GL_BACK);
            // This is the stall: it waits for the GPU to finish everything
            // queued before it can hand the pixels back. Unavoidable on this
            // path, and the reason it is only a diagnostic rather than the
            // default.
            gl::glReadPixels(
                0,
                0,
                rect.width as i32,
                rect.height as i32,
                GL_BGRA,
                gl::GL_UNSIGNED_BYTE,
                self.pixels.as_mut_ptr() as *mut _,
            );
        }
        let pixels = &self.pixels;

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { context.Map(&self.texture, 0, D3D11_MAP_WRITE, 0, Some(&mut mapped)) }
            .map_err(|err| format!("Map of the readback texture failed: {err}"))?;

        for row in 0..rect.height as usize {
            // glReadPixels hands back bottom-up rows; a D3D11 texture is
            // top-down, so the source row is normally counted from the end.
            let source_row = if flip_y {
                rect.height as usize - 1 - row
            } else {
                row
            };
            let Some(source) = pixels.get(source_row * stride..source_row * stride + stride) else {
                continue;
            };
            let destination =
                (rect.y as usize + row) * mapped.RowPitch as usize + rect.x as usize * 4;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    source.as_ptr(),
                    (mapped.pData as *mut u8).add(destination),
                    stride,
                );
            }
        }

        unsafe { context.Unmap(&self.texture, 0) };
        Ok(())
    }
}

fn slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    text: &str,
) -> bool {
    ui.add(
        egui::Slider::new(value, range)
            .text(text)
            .step_by(0.01)
            .clamping(egui::widgets::SliderClamping::Always),
    )
    .changed()
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.pipe.send(&proto::ClientMessage::Goodbye);
        unsafe { gl::glDeleteFramebuffers(1, &self.draw_fbo) };
    }
}

/// Staging texture descriptions to try, in order, until the driver takes one.
///
/// `wglDXRegisterObjectNV` is picky in ways that vary by driver and are not
/// discoverable except by asking, so this asks. Each entry is a format, the
/// `MiscFlags` to create it with.
///
/// The format must stay in the `B8G8R8A8` family: `CopySubresourceRegion` into
/// the compositor's `B8G8R8A8_UNORM_SRGB` texture is only legal between formats
/// that share a typeless parent, so `R8G8B8A8` is not an option however well
/// the driver might like it.
const STAGING_CANDIDATES: &[(DXGI_FORMAT, u32)] = &[
    (DXGI_FORMAT_B8G8R8A8_UNORM, 0),
    (DXGI_FORMAT_B8G8R8A8_UNORM, D3D11_RESOURCE_MISC_SHARED),
    (DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, 0),
];

/// `D3D11_RESOURCE_MISC_SHARED`.
const D3D11_RESOURCE_MISC_SHARED: u32 = 0x2;

impl Staging {
    fn new(device: &ID3D11Device, layout: &proto::TextureLayout) -> Result<Self, String> {
        let wgl = Wgl::load()?;

        let interop_device = unsafe { (wgl.open_device)(device.as_raw()) };
        if interop_device.is_invalid() {
            return Err(
                "wglDXOpenDeviceNV failed. The driver claims WGL_NV_DX_interop2 but \
                 will not open the D3D11 device; this is the layer that also produces the \
                 known AMD failures."
                    .to_owned(),
            );
        }

        let mut tried = Vec::new();
        for (format, misc) in STAGING_CANDIDATES {
            match Self::try_register(device, &wgl, interop_device, layout, *format, *misc) {
                Ok((texture, gl_name, object)) => {
                    return Ok(Self {
                        texture,
                        gl_name,
                        object,
                        interop_device,
                        wgl,
                    });
                }
                Err(err) => {
                    tried.push(format!(
                        "  format {:#x}, MiscFlags {misc:#x} -- {err}",
                        format.0
                    ));
                }
            }
        }

        unsafe { (wgl.close_device)(interop_device) };
        Err(format!(
            "no staging texture this driver will register with WGL_NV_DX_interop2. That \
             extension is the only way OpenGL pixels reach a D3D11 texture here, so without it \
             this app cannot feed the compositor.\nTried:\n{}",
            tried.join("\n")
        ))
    }

    /// Create one candidate texture and try to register it.
    ///
    /// Everything is cleaned up on failure, so the caller can simply try the
    /// next one.
    fn try_register(
        device: &ID3D11Device,
        wgl: &Wgl,
        interop_device: HANDLE,
        layout: &proto::TextureLayout,
        format: DXGI_FORMAT,
        misc: u32,
    ) -> Result<(ID3D11Texture2D, gl::GLuint, HANDLE), String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: layout.width,
            Height: layout.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            // The texture is used as a GL texture and as a D3D copy source.
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            CPUAccessFlags: 0,
            // Never a keyed mutex: registering one of those is what this whole
            // texture exists to avoid. See the module docs.
            MiscFlags: misc,
        };
        let mut texture = None;
        unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture)) }
            .map_err(|err| format!("CreateTexture2D failed: {err}"))?;
        let texture: ID3D11Texture2D = texture.ok_or("CreateTexture2D returned nothing")?;

        let mut gl_name: gl::GLuint = 0;
        unsafe { gl::glGenTextures(1, &mut gl_name) };
        if gl_name == 0 {
            return Err("glGenTextures returned zero".to_owned());
        }

        // Configure the GL object before registration. The D3D11 texture owns
        // the storage; no glTexImage2D call is made here.
        unsafe {
            gl::glBindTexture(gl::GL_TEXTURE_2D, gl_name);
            gl::glTexParameteri(
                gl::GL_TEXTURE_2D,
                gl::GL_TEXTURE_MIN_FILTER,
                gl::GL_LINEAR as i32,
            );
            gl::glTexParameteri(
                gl::GL_TEXTURE_2D,
                gl::GL_TEXTURE_MAG_FILTER,
                gl::GL_LINEAR as i32,
            );
            gl::glTexParameteri(
                gl::GL_TEXTURE_2D,
                gl::GL_TEXTURE_WRAP_S,
                gl::GL_CLAMP_TO_EDGE as i32,
            );
            gl::glTexParameteri(
                gl::GL_TEXTURE_2D,
                gl::GL_TEXTURE_WRAP_T,
                gl::GL_CLAMP_TO_EDGE as i32,
            );
            gl::glBindTexture(gl::GL_TEXTURE_2D, 0);
        }

        let object = unsafe {
            (wgl.register_object)(
                interop_device,
                texture.as_raw(),
                gl_name,
                gl::GL_TEXTURE_2D,
                WGL_ACCESS_READ_WRITE_NV,
            )
        };
        if object.is_invalid() {
            unsafe { gl::glDeleteTextures(1, &gl_name) };
            return Err("wglDXRegisterObjectNV refused it".to_owned());
        }
        Ok((texture, gl_name, object))
    }

    fn lock(&self) -> Result<(), String> {
        let mut object = self.object;
        let ok = unsafe { (self.wgl.lock_objects)(self.interop_device, 1, &mut object) };
        if ok == 0 {
            return Err("wglDXLockObjectsNV failed".to_owned());
        }
        Ok(())
    }

    fn unlock(&self) -> Result<(), String> {
        let mut object = self.object;
        let ok = unsafe { (self.wgl.unlock_objects)(self.interop_device, 1, &mut object) };
        if ok == 0 {
            return Err("wglDXUnlockObjectsNV failed".to_owned());
        }
        Ok(())
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        unsafe {
            if !self.object.is_invalid() {
                (self.wgl.unregister_object)(self.interop_device, self.object);
            }
            if !self.interop_device.is_invalid() {
                (self.wgl.close_device)(self.interop_device);
            }
            gl::glDeleteTextures(1, &self.gl_name);
        }
    }
}

/// `wglDXOpenDeviceNV`.
type OpenDevice = unsafe extern "system" fn(*mut c_void) -> HANDLE;
/// `wglDXCloseDeviceNV`.
type CloseDevice = unsafe extern "system" fn(HANDLE) -> i32;
/// `wglDXRegisterObjectNV`.
type RegisterObject =
    unsafe extern "system" fn(HANDLE, *mut c_void, gl::GLuint, gl::GLenum, gl::GLenum) -> HANDLE;
/// `wglDXUnregisterObjectNV`.
type UnregisterObject = unsafe extern "system" fn(HANDLE, HANDLE) -> i32;
/// `wglDXLockObjectsNV` and `wglDXUnlockObjectsNV`, which share a signature.
type LockObjects = unsafe extern "system" fn(HANDLE, i32, *mut HANDLE) -> i32;

/// The `WGL_NV_DX_interop2` entry points, looked up at run time.
///
/// They are extension functions, so they exist only through
/// `wglGetProcAddress` and only once a context is current.
struct Wgl {
    open_device: OpenDevice,
    close_device: CloseDevice,
    register_object: RegisterObject,
    unregister_object: UnregisterObject,
    lock_objects: LockObjects,
    unlock_objects: LockObjects,
}

impl Wgl {
    fn load() -> Result<Self, String> {
        // Safety: `wglGetProcAddress` is untyped, so each pointer is transmuted
        // to the signature the extension specifies -- written out in full above
        // rather than inferred, since a mismatch here would be a silent ABI bug
        // rather than a compile error. Only reached once the address is known
        // to be non-null.
        unsafe {
            Ok(Self {
                open_device: std::mem::transmute::<*const c_void, OpenDevice>(proc_address(
                    "wglDXOpenDeviceNV",
                )?),
                close_device: std::mem::transmute::<*const c_void, CloseDevice>(proc_address(
                    "wglDXCloseDeviceNV",
                )?),
                register_object: std::mem::transmute::<*const c_void, RegisterObject>(
                    proc_address("wglDXRegisterObjectNV")?,
                ),
                unregister_object: std::mem::transmute::<*const c_void, UnregisterObject>(
                    proc_address("wglDXUnregisterObjectNV")?,
                ),
                lock_objects: std::mem::transmute::<*const c_void, LockObjects>(proc_address(
                    "wglDXLockObjectsNV",
                )?),
                unlock_objects: std::mem::transmute::<*const c_void, LockObjects>(proc_address(
                    "wglDXUnlockObjectsNV",
                )?),
            })
        }
    }
}

fn proc_address(name: &str) -> Result<*const c_void, String> {
    let cname = std::ffi::CString::new(name).expect("no interior nul in a literal");
    let address = unsafe { wglGetProcAddress(cname.as_ptr()) };
    if address.is_null() {
        return Err(format!(
            "this OpenGL driver does not provide {name}, so it does not support \
             WGL_NV_DX_interop2. That extension is how OpenGL pixels reach a D3D11 texture; \
             without it this app cannot talk to the compositor."
        ));
    }
    Ok(address)
}

fn force_readback() -> bool {
    std::env::var("PORTAL_VR_READBACK")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

fn interop_test_pattern() -> bool {
    std::env::var("PORTAL_VR_INTEROP_TEST_PATTERN")
        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

#[link(name = "opengl32")]
unsafe extern "system" {
    fn wglGetProcAddress(name: *const std::os::raw::c_char) -> *const c_void;
}

/// Create a D3D11 device on the adapter the compositor named.
fn create_device(luid: i64) -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    let adapter = find_adapter(luid)?;
    let levels = [D3D_FEATURE_LEVEL_11_0];
    let mut device = None;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            &adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            Default::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .map_err(|err| format!("D3D11CreateDevice failed on adapter {luid:#x}: {err}"))?;
    Ok((
        device.ok_or("D3D11CreateDevice returned no device")?,
        context.ok_or("D3D11CreateDevice returned no context")?,
    ))
}

fn find_adapter(luid: i64) -> Result<IDXGIAdapter, String> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
        .map_err(|err| format!("CreateDXGIFactory1 failed: {err}"))?;
    for index in 0.. {
        let adapter = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(err) => return Err(format!("EnumAdapters1({index}) failed: {err}")),
        };
        let desc = unsafe { adapter.GetDesc1() }
            .map_err(|err| format!("GetDesc1({index}) failed: {err}"))?;
        let this = ((desc.AdapterLuid.HighPart as i64) << 32) | (desc.AdapterLuid.LowPart as i64);
        if this == luid {
            return Ok(adapter.cast().expect("IDXGIAdapter1 is an IDXGIAdapter"));
        }
    }
    Err(format!(
        "no display adapter has LUID {luid:#x}, which is the one the compositor is on. Both \
         processes must use the same GPU."
    ))
}

fn open_slot(
    device: &ID3D11Device1,
    slot: &proto::SlotHandles,
    index: usize,
) -> Result<Slot, String> {
    let texture: ID3D11Texture2D =
        unsafe { device.OpenSharedResource1(HANDLE(slot.texture as *mut c_void)) }
            .map_err(|err| format!("OpenSharedResource1 failed for slot {index}: {err}"))?;
    let mutex: IDXGIKeyedMutex = texture
        .cast()
        .map_err(|err| format!("slot {index} has no keyed mutex: {err}"))?;
    Ok(Slot { texture, mutex })
}

/// Take a keyed mutex, reporting a timeout as `false` rather than an error.
///
/// Called through the vtable rather than through `IDXGIKeyedMutex::AcquireSync`
/// on purpose. The safe wrapper ends in `HRESULT::ok()`, which treats every
/// non-negative code as success -- but `AcquireSync` reports failure with two
/// *positive* codes, `WAIT_TIMEOUT` (0x102) and `WAIT_ABANDONED` (0x80).
/// Through the wrapper both look like success, and this app would go on to draw
/// into a texture the compositor is still reading. That is a torn frame with no
/// error anywhere.
fn acquire(mutex: &IDXGIKeyedMutex, key: u64, timeout_ms: u32) -> Result<bool, String> {
    let hresult = unsafe {
        (Interface::vtable(mutex).AcquireSync)(Interface::as_raw(mutex), key, timeout_ms)
    };
    match hresult.0 as u32 {
        0 => Ok(true),
        WAIT_TIMEOUT_HRESULT => Ok(false),
        // The compositor died holding it. The contents are undefined, but the
        // mutex is ours now and still has to be released.
        WAIT_ABANDONED_HRESULT => Ok(true),
        _ => Err(format!(
            "AcquireSync({key}) failed: {}",
            windows::core::Error::from_hresult(hresult)
        )),
    }
}

fn release(mutex: &IDXGIKeyedMutex, key: u64) -> Result<(), String> {
    unsafe { mutex.ReleaseSync(key) }.map_err(|err| format!("ReleaseSync({key}) failed: {err}"))
}

/// The named pipe to the compositor.
///
/// Synchronous, and used from the render thread only: unlike the compositor,
/// which reads and writes from two threads and therefore needs overlapped I/O,
/// this side never has a read and a write outstanding at once.
struct Pipe(HANDLE);

impl Pipe {
    fn connect() -> Result<Self, String> {
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_MODE, OPEN_EXISTING,
        };
        let name: Vec<u16> = proto::PIPE_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                windows::core::PCWSTR(name.as_ptr()),
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .map_err(|err| {
            format!(
                "could not connect to the compositor on {}: {err}. Start portal-compositor \
                 first.",
                proto::PIPE_NAME
            )
        })?;
        Ok(Self(handle))
    }

    fn send(&mut self, message: &proto::ClientMessage) -> Result<(), String> {
        use std::io::Write;
        let bytes = proto::encode(message).map_err(|err| err.to_string())?;
        Stream(self.0)
            .write_all(&bytes)
            .map_err(|err| format!("could not send to the compositor: {err}"))
    }

    /// Read one message, waiting for it. Only used for the handshake.
    fn recv_blocking(&mut self) -> Result<proto::ServerMessage, String> {
        proto::decode(&mut Stream(self.0)).map_err(|err| err.to_string())
    }

    /// Read one message if one has already arrived.
    ///
    /// `PeekNamedPipe` rather than a blocking read, because this runs on the
    /// render thread: waiting on the compositor here would make the app's own
    /// window stutter whenever the compositor was busy, which is exactly the
    /// coupling the two-process design exists to remove.
    fn try_recv(&mut self) -> Result<Option<proto::ServerMessage>, String> {
        use windows::Win32::System::Pipes::PeekNamedPipe;
        let mut available = 0u32;
        unsafe { PeekNamedPipe(self.0, None, 0, None, Some(&mut available), None) }
            .map_err(|err| format!("the compositor closed the connection: {err}"))?;
        // A length prefix is four bytes; less than that is not a message yet.
        if available < 4 {
            return Ok(None);
        }
        proto::decode(&mut Stream(self.0))
            .map(Some)
            .map_err(|err| err.to_string())
    }
}

impl Drop for Pipe {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

/// `Read`/`Write` over the pipe handle, which [`Pipe`] owns.
struct Stream(HANDLE);

impl std::io::Read for Stream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        use windows::Win32::Storage::FileSystem::ReadFile;
        let mut read = 0u32;
        unsafe { ReadFile(self.0, Some(buffer), Some(&mut read), None) }.map_err(|err| {
            std::io::Error::from_raw_os_error((err.code().0 as u32 & 0xffff) as i32)
        })?;
        Ok(read as usize)
    }
}

impl std::io::Write for Stream {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        use windows::Win32::Storage::FileSystem::WriteFile;
        let mut written = 0u32;
        unsafe { WriteFile(self.0, Some(buffer), Some(&mut written), None) }.map_err(|err| {
            std::io::Error::from_raw_os_error((err.code().0 as u32 & 0xffff) as i32)
        })?;
        Ok(written as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn log(message: String) {
    eprintln!("[vr] {message}");
}
