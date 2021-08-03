use eww::{egui, wgpu, winit};

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
        window: &window,
        device: &wgpu.device,
        rt_format: wgpu::TextureFormat::Bgra8UnormSrgb,
        style: egui::Style::default(),
        font_definitions: egui::FontDefinitions::default(),
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

    let frame = match wgpu.swap_chain.get_current_frame() {
        Ok(frame) => frame,
        Err(e) => {
            eprintln!("wgpu error: {}", e);
            return;
        }
    };
    let rt = &frame.output.view;

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
}

fn build_gui(ctx: egui::CtxRef) {
    egui::Window::new("eww basic example").show(&ctx, |ui| {
        ui.label("This is a basic example of eww.");
    });
}

struct WgpuCtx {
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: wgpu::SwapChain,
}

impl WgpuCtx {
    async fn init(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
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

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        Self {
            device,
            queue,
            swap_chain,
        }
    }
}
