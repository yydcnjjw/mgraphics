use std::borrow::Cow;

use wgpu::RenderPipeline;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[cfg(target_os = "windows")]
use winit::platform::windows::{EventLoopExtWindows, WindowBuilderExtWindows};

#[cfg(target_os = "linux")]
use winit::platform::unix::{EventLoopExtUnix, WindowBuilderExtUnix, XWindowType};

fn create_window(event_loop: &EventLoop<()>) -> anyhow::Result<Window> {
    #[allow(unused_mut)]
    let mut window_builder = WindowBuilder::new();

    #[cfg(target_os = "linux")]
    {
        window_builder = window_builder.with_x11_window_type(vec![XWindowType::Toolbar]);
    }

    let primary = event_loop
        .primary_monitor()
        .ok_or(anyhow::anyhow!("primary monitor is not found"))?;

    let primary_size = primary.size();

    let size = PhysicalSize::<u32>::new(1024, 128);

    let pos = PhysicalPosition::<i32>::new(((primary_size.width - size.width) / 2).try_into()?, 8);

    let window = window_builder
        .with_position(pos)
        .with_inner_size(size)
        // .with_decorations(false)
        .with_transparent(true)
        .build(&event_loop)?;
    Ok(window)
}

async fn run() -> anyhow::Result<()> {
    let event_loop = EventLoop::<()>::new_any_thread();

    let window = create_window(&event_loop)?;

    let mut ctx = RenderContext::new(&window).await;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                ctx.surface_config.width = size.width;
                ctx.surface_config.height = size.height;
                ctx.surface.configure(&ctx.device, &ctx.surface_config);
            }
            Event::RedrawRequested(_) => {
                draw(&ctx);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}

struct RenderContext {
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: RenderPipeline,
}

impl RenderContext {
    async fn new(window: &Window) -> Self {
        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(if cfg!(windows) {
            wgpu::Backends::DX12
        } else {
            wgpu::Backends::PRIMARY
        });

        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };

        surface.configure(&device, &surface_config);

        RenderContext {
            surface,
            surface_config,
            device,
            queue,
            render_pipeline,
        }
    }
}

fn draw(ctx: &RenderContext) {
    let frame = ctx
        .surface
        .get_current_texture()
        .expect("Failed to acquire next swap chain texture");
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        rpass.set_pipeline(&ctx.render_pipeline);
        rpass.draw(0..3, 0..1);
    }

    ctx.queue.submit(Some(encoder.finish()));
    frame.present();
}

#[tokio::main]
async fn main() {
    run().await.unwrap();
}
