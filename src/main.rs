use futures::executor::block_on;
use imgui::*;
use imgui_wgpu::Renderer as ImGuiRenderer;
use imgui_winit_support;
use std::time::Instant;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = block_on(State::new(&window));

    let mut hidpi_factor = window.scale_factor();

    // let mut imgui = imgui::Context::create();
    // let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    // platform.attach_window(
    //     imgui.io_mut(),
    //     &window,
    //     imgui_winit_support::HiDpiMode::Default,
    // );
    // imgui.set_ini_filename(None);

    // let font_size = (13.0 * hidpi_factor) as f32;
    // imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    // imgui.fonts().add_font(&[FontSource::DefaultFontData {
    //     config: Some(imgui::FontConfig {
    //         oversample_h: 1,
    //         pixel_snap_h: true,
    //         size_pixels: font_size,
    //         ..Default::default()
    //     }),
    // }]);

    // let mut renderer = Renderer::new(&mut imgui, &device, &mut queue, sc_desc.format);

    let mut last_frame = Instant::now();
    let mut demo_open = true;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                let delta_s = last_frame.elapsed();
                state.update();
                state.render(&window);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            _ => {}
                        },
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        state
            .platform
            .handle_event(state.imgui_context.io_mut(), &window, &event);
    });
}

struct State {
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline,
    size: winit::dpi::PhysicalSize<u32>,

    // IMGUI stuff
    pub imgui_context: imgui::Context,
    imgui_renderer: ImGuiRenderer,
    pub platform: imgui_winit_support::WinitPlatform,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        let (device, mut queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await
            .unwrap();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let vs_module = device.create_shader_module(wgpu::include_spirv!("shader.vert.spv"));
        let fs_module = device.create_shader_module(wgpu::include_spirv!("shader.frag.spv"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                clamp_depth: false,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: sc_desc.format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let mut imgui_context = imgui::Context::create();
        imgui_context.io_mut().config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;

        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui_context);
        platform.attach_window(
            imgui_context.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );
        imgui_context.set_ini_filename(None);

        let hidpi_factor = 1.0;
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui_context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui_context
            .fonts()
            .add_font(&[FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        #[cfg(not(feature = "glsl-to-spirv"))]
        let mut imgui_renderer =
            ImGuiRenderer::new(&mut imgui_context, &device, &mut queue, sc_desc.format);

        #[cfg(feature = "glsl-to-spirv")]
        let mut imgui_renderer =
            ImGuiRenderer::new_glsl(&mut imgui_context, &device, &mut queue, sc_desc.format);

        Self {
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swap_chain,
            render_pipeline,
            size,

            // IMGUI stuff
            imgui_context,
            imgui_renderer,
            platform,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self, window: &Window) {
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("Timeout getting texture")
            .output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);

            self.platform
                .prepare_frame(self.imgui_context.io_mut(), &window)
                .expect("Failed to prepare frame");

            let ui = self.imgui_context.frame();

            let mut var = true;
            let demo_window = ui.show_demo_window(&mut var);

            let window_one = imgui::Window::new(im_str!("Hello world"));
            window_one
                .size([300.0, 100.0], Condition::FirstUseEver)
                .build(&ui, || {
                    ui.text(im_str!("Hello world!"));
                    ui.text(im_str!("This...is...imgui-rs on WGPU!"));
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(im_str!(
                        "Mouse Position: ({:.1},{:.1})",
                        mouse_pos[0],
                        mouse_pos[1]
                    ));
                });

            let window_two = imgui::Window::new(im_str!("Hello too"));
            window_two
                .size([400.0, 200.0], Condition::FirstUseEver)
                .position([400.0, 200.0], Condition::FirstUseEver)
                .build(&ui, || {
                    ui.text(im_str!("Frametime: {:?}", 0.0));
                });

            let width = self.size.width as f32;
            let height = self.size.height as f32;
            imgui::Dock::new().build(|root| {
                root.size([width - 100.0, height - 100.0])
                    .position([0_f32, 0_f32])
                    .split(
                        imgui::Direction::Left,
                        0.7_f32,
                        |left| {
                            left.dock_window(im_str!("Dear ImGui Demo"));
                            left.dock_window(im_str!("Hello world"));
                        },
                        |right| {
                            right.dock_window(im_str!("Hello too"));
                        },
                    )
            });

            let main_window = imgui::Window::new(im_str!("Main Window"));
            main_window
                .size([width, height], Condition::FirstUseEver)
                .position([0.0, 0.0], Condition::FirstUseEver)
                .build(&ui, || {});

            self.imgui_renderer
                .render(ui.render(), &self.queue, &self.device, &mut render_pass)
                .expect("Rendering failed");
        }

        self.queue.submit(Some(encoder.finish()));
    }
}
