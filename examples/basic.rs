use eww::wgpu::{Backends, TextureUsages};
use eww::{egui, wgpu, winit};
use eww::egui::Context;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use futures::executor::block_on;

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_resizable(false)
        .with_title("eww basic example")
        .build(&event_loop)
        .unwrap();

    let wgpu = block_on(WgpuCtx::init(&window));

    let mut backend = eww::Backend::new(eww::BackendDescriptor {
        device: &wgpu.device,
        rt_format: wgpu::TextureFormat::Bgra8UnormSrgb,
        event_loop: &event_loop,
    });

    event_loop.run(move |event, _, control_flow| {
        backend.handle_event(&event);

        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => render(&wgpu, &window, &mut backend),

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn render(wgpu: &WgpuCtx, window: &Window, backend: &mut eww::Backend) {
    let mut encoder = wgpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    let frame = match wgpu.surface.get_current_texture() {
        Ok(frame) => frame,
        Err(e) => {
            eprintln!("wgpu error: {}", e);
            return;
        }
    };
    let rt = &frame.texture.create_view(&wgpu::TextureViewDescriptor {
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
        label: None,
        format: None,
        dimension: None,
    });

    backend.render(
        eww::RenderDescriptor {
            textures_to_update: &[],
            window,
            device: &wgpu.device,
            queue: &wgpu.queue,
            encoder: &mut encoder,
            render_target: rt,
            load_operation: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        },
        |ctx| {
            build_gui(ctx);
        },
    );

    wgpu.queue.submit(Some(encoder.finish()));
    frame.present();
}

fn build_gui(ctx: &Context) {
    egui::Window::new("eww basic example").show(ctx, |ui| {
        ui.label("This is a basic example of eww.");
    });
}

struct WgpuCtx {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
}

impl WgpuCtx {
    async fn init(window: &Window) -> Self {
        let instance = wgpu::Instance::new(Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let window_size = window.inner_size();

        surface.configure(&device, &wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
        });

        Self {
            device,
            queue,
            surface,
        }
    }
}
