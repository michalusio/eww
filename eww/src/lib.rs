pub use egui_wgpu as renderer;
pub use egui_winit as platform;

pub use platform::egui; // same as renderer::egui
pub use platform::winit;
pub use renderer::wgpu;

use platform::{Platform, PlatformDescriptor};
use renderer::{Renderer, RendererDescriptor};

use winit::window;

/// Egui backend with winit platform and wgpu renderer
pub struct Backend {
    platform: Platform,
    renderer: Renderer,
}

impl Backend {
    pub fn new(desc: BackendDescriptor) -> Self {
        let BackendDescriptor {
            window,
            device,
            rt_format,
            style,
            font_definitions,
        } = desc;

        let platform = Platform::new(PlatformDescriptor {
            window,
            style,
            font_definitions,
        });

        let renderer = Renderer::new(RendererDescriptor { device, rt_format });

        Self { platform, renderer }
    }

    pub fn handle_event<T>(&mut self, event: &winit::event::Event<T>) {
        self.platform.handle_event(event);
    }

    // TODO: is this better than Self::render() taking a closure?
    // It would be interesting if you could continue building the ui after ending (pausing) a frame.
    //pub fn begin_frame(&mut self) {

    //}

    //pub fn end_frame(&mut self) {
    //}

    pub fn render<F>(&mut self, desc: RenderDescriptor, build_ui: F)
    where
        F: FnOnce(egui::CtxRef),
    {
        let RenderDescriptor {
            textures_to_update,
            window,
            device,
            queue,
            encoder,
            render_target,
            load_operation,
        } = desc;

        let screen_descriptor = {
            let size = window.inner_size();
            renderer::ScreenDescriptor {
                physical_width: size.width,
                physical_height: size.height,
                scale_factor: window.scale_factor() as f32,
            }
        };

        self.platform.begin_frame();
        build_ui(self.ctx());
        let (shapes, needs_redraw) = self.platform.end_frame(window);

        let _ = needs_redraw; // TODO use

        let meshes = &self.ctx().tessellate(shapes);

        // this is a mess.
        // TODO: use iterators in the first place instead of slices
        let textures_to_update = textures_to_update.iter().copied();
        let egui_texture = self.ctx().texture();
        let egui_texture = std::iter::once(egui_texture.as_ref());
        let textures_to_update: Vec<&egui::Texture> =
            textures_to_update.chain(egui_texture).collect();

        self.renderer.render(renderer::RenderDescriptor {
            meshes,
            textures_to_update: &textures_to_update,
            device,
            queue,
            encoder,
            render_target,
            screen_descriptor,
            load_operation,
        });
    }

    pub fn ctx(&self) -> egui::CtxRef {
        self.platform.ctx()
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
pub struct BackendDescriptor<'a> {
    /// Winit window
    pub window: &'a window::Window,
    /// Wgpu device
    pub device: &'a wgpu::Device,
    /// Render target format
    pub rt_format: wgpu::TextureFormat,
    /// Egui style configuration.
    pub style: egui::Style,
    /// Egui font configuration.
    pub font_definitions: egui::FontDefinitions,
}

pub struct RenderDescriptor<'a> {
    // TODO: turn into iterator
    pub textures_to_update: &'a [&'a egui::Texture],
    pub window: &'a window::Window,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub render_target: &'a wgpu::TextureView,
    pub load_operation: wgpu::LoadOp<wgpu::Color>,
}
