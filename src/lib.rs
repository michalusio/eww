pub mod render_target;

pub use egui_wgpu as renderer;
pub use egui_winit as platform;

pub use platform::egui;
pub use platform::winit;
pub use renderer::wgpu;

pub use platform::State as Platform;
pub use renderer::renderer::RenderPass as Renderer;

use egui::Context as Ctx;
use winit::window;

/// Egui backend with winit platform and wgpu renderer
pub struct Backend {
    ctx: Ctx,
    platform: Platform,
    renderer: Renderer,
}

impl Backend {
    pub fn new<T>(desc: BackendDescriptor<T>) -> Self {
        let BackendDescriptor {
            event_loop,
            device,
            rt_format,
        } = desc;

        let platform = Platform::new(event_loop);

        let renderer = Renderer::new(device, rt_format, 0);

        let ctx = Ctx::default();

        Self {
            ctx,
            platform,
            renderer,
        }
    }

    // output indicates if egui wants exclusive access to this event
    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) -> bool {
        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                self.platform.on_event(&self.ctx, event)
            }
            _ => false,
        }
    }

    // TODO: is this better than Self::render() taking a closure?
    // It would be interesting if you could continue building the ui after ending (pausing) a frame.
    //pub fn begin_frame(&mut self) {

    //}

    //pub fn end_frame(&mut self) {
    //}

    pub fn render<F>(&mut self, desc: RenderDescriptor, build_ui: F)
    where
        F: FnOnce(&Ctx),
    {
        let RenderDescriptor {
            // TODO: use
            textures_to_update: _,
            window,
            device,
            queue,
            encoder,
            render_target,
            load_operation,
        } = desc;

        let screen_descriptor = {
            let size = window.inner_size();
            renderer::renderer::ScreenDescriptor {
                size_in_pixels: [size.width, size.height],
                pixels_per_point: window.scale_factor() as f32,
            }
        };

        let raw_input: egui::RawInput = self.platform.take_egui_input(window);
        let full_output = self.ctx.run(raw_input, |ctx| {
            build_ui(ctx);
        });
        self.platform
            .handle_platform_output(window, &self.ctx, full_output.platform_output);

        let clipped_primitives = self.ctx().tessellate(full_output.shapes);

        self.renderer
            .update_buffers(device, queue, &clipped_primitives, &screen_descriptor);
        for (tex_id, img_delta) in full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, tex_id, &img_delta);
        }
        for tex_id in full_output.textures_delta.free {
            self.renderer.free_texture(&tex_id);
        }

        let clear_color = match load_operation {
            wgpu::LoadOp::Clear(c) => Some(c),
            wgpu::LoadOp::Load => None,
        };

        self.renderer.execute(
            encoder,
            render_target,
            &clipped_primitives,
            &screen_descriptor,
            clear_color,
        );
    }

    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    pub fn platform(&self) -> &Platform {
        &self.platform
    }

    pub fn platform_mut(&mut self) -> &mut Platform {
        &mut self.platform
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }
}

/// Backend creation descriptor
pub struct BackendDescriptor<'a, T: 'static> {
    /// Winit window
    pub event_loop: &'a winit::event_loop::EventLoop<T>,
    /// Wgpu device
    pub device: &'a wgpu::Device,
    /// Render target format
    pub rt_format: wgpu::TextureFormat,
}

pub struct RenderDescriptor<'a> {
    // TODO: turn into iterator
    pub textures_to_update: &'a [&'a egui::TextureId],
    pub window: &'a window::Window,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub render_target: &'a wgpu::TextureView,
    pub load_operation: wgpu::LoadOp<wgpu::Color>,
}
