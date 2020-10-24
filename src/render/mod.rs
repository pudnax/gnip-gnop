use crate::include_str_from_outdir;
use eyre::*;
use wgpu::{util::*, *};
use wgpu_glyph::{ab_glyph, Section, Text};
use winit::{monitor::VideoMode, window::Window};

use crate::math::Vec2;
use crate::state;

mod buffers;
use buffers::*;

pub const SHADER_ENTRY_POINT_NAME: &str = "main";

const FONT_BYTES: &[u8] = include_bytes!("../../res/fonts/PressStart2P-Regular.ttf");

#[rustfmt::skip]
const VERTEXES: &[Vertex; 6] = &[
    Vertex { pos: Vec2 { x:  0.0, y:  0.5 } },
    Vertex { pos: Vec2 { x: -0.5, y: -0.5 } },
    Vertex { pos: Vec2 { x: -0.5, y:  0.5 } },
    Vertex { pos: Vec2 { x:  0.0, y: -0.5 } },
    Vertex { pos: Vec2 { x:  0.5, y: -0.5 } },
    Vertex { pos: Vec2 { x:  0.5, y:  0.5 } },

];

const INDEXES: &[u32; 6] = &[0, 2, 1, 3, 4, 5];

pub struct Renderer {
    surface: Surface,
    adapter: Adapter,
    device: Device,
    queue: Queue,

    swap_chain: SwapChain,
    sc_desc: SwapChainDescriptor,

    render_pipeline: RenderPipeline,
    rp_layout: PipelineLayout,

    basic_vertex_buffer: Buffer,
    basic_index_buffer: Buffer,
    num_indices: u32,

    vertex_buffer: Buffer,
    index_buffer: Buffer,

    glyph_brush: wgpu_glyph::GlyphBrush<()>,
    staging_belt: StagingBelt,
}

impl Renderer {
    pub fn width(&self) -> f32 {
        self.sc_desc.width as f32
    }

    #[allow(dead_code)]
    pub fn height(&self) -> f32 {
        self.sc_desc.height as f32
    }

    pub async fn new(window: &Window, video_mode: &VideoMode) -> Result<Self> {
        let backend_bit = BackendBit::PRIMARY;
        let instance = Instance::new(backend_bit);
        println!(
            "All available adapters that match {:?} backends: {:?}",
            backend_bit,
            instance
                .enumerate_adapters(BackendBit::PRIMARY)
                .collect::<Vec<_>>()
        );

        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::Default,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or_else(|| {
                eyre!(
                    "Failed to provide adapter for the {:?} backend bit.",
                    backend_bit
                )
            })?;

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    features: Features::default(),
                    limits: Limits::default(),
                    shader_validation: true,
                },
                Some(std::path::Path::new(env!("OUT_DIR"))),
            )
            .await?;

        let size = video_mode.size();
        let sc_desc = SwapChainDescriptor {
            usage: TextureUsage::OUTPUT_ATTACHMENT,
            present_mode: PresentMode::Fifo,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let rp_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Main Render Pipeline Layout Descriptor"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let basic_vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            usage: BufferUsage::VERTEX,
            contents: bytemuck::cast_slice(VERTEXES),
        });

        let basic_index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            usage: BufferUsage::INDEX,
            contents: bytemuck::cast_slice(INDEXES),
        });
        let num_indices = INDEXES.len() as u32;

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: Vertex::SIZE * 4 * 3,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: U32_SIZE * 6 * 3,
            usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let mut shader_compiler =
            shaderc::Compiler::new().ok_or_else(|| eyre!("Failed to create shader compiler."))?;
        let render_pipeline = {
            let vs_src = include_str_from_outdir!("/shaders/shader.vert");
            let fs_src = include_str_from_outdir!("/shaders/shader.frag");
            let vs_spirv = shader_compiler
                .compile_into_spirv(
                    vs_src,
                    shaderc::ShaderKind::Vertex,
                    "shader.vert",
                    SHADER_ENTRY_POINT_NAME,
                    None,
                )
                .unwrap();
            let fs_spirv = shader_compiler
                .compile_into_spirv(
                    fs_src,
                    shaderc::ShaderKind::Fragment,
                    "shader.frag",
                    SHADER_ENTRY_POINT_NAME,
                    None,
                )
                .unwrap();
            let vs_module =
                device.create_shader_module(wgpu::util::make_spirv(&vs_spirv.as_binary_u8()));
            let fs_module =
                device.create_shader_module(wgpu::util::make_spirv(&fs_spirv.as_binary_u8()));

            create_render_pipeline(
                &device,
                &rp_layout,
                sc_desc.format,
                &[Vertex::DESC],
                vs_module,
                fs_module,
            )
        };

        let font = ab_glyph::FontArc::try_from_slice(FONT_BYTES).unwrap();
        let glyph_brush =
            wgpu_glyph::GlyphBrushBuilder::using_font(font).build(&device, sc_desc.format);
        let staging_belt = wgpu::util::StagingBelt::new(1024);

        Ok(Self {
            surface,
            adapter,
            device,
            queue,
            swap_chain,
            sc_desc,
            render_pipeline,
            rp_layout,

            basic_vertex_buffer,
            basic_index_buffer,
            num_indices,

            vertex_buffer,
            index_buffer,

            glyph_brush,
            staging_belt,
        })
    }

    pub fn render(&mut self) -> Result<()> {
        let frame = match self.swap_chain.get_current_frame() {
            Ok(frame) => Ok(frame.output),
            Err(wgpu::SwapChainError::Outdated) => {
                self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
                return Ok(());
            }
            Err(e) => Err(e),
        }?;

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
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
        render_pass.set_vertex_buffer(0, self.basic_vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.basic_index_buffer.slice(..));
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn render_state(&mut self, state: &state::State) -> Result<()> {
        let frame = match self.swap_chain.get_current_frame() {
            Ok(frame) => Ok(frame.output),
            Err(wgpu::SwapChainError::Outdated) => {
                self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
                return Ok(());
            }
            Err(e) => Err(e),
        }?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("State Command Encoder"),
            });

        let num_indices = if state.ball.visible || state.player1.visible || state.player2.visible {
            let (stg_vertex, stg_index, num_indices) = QuadBufferBuilder::new()
                .push_ball(&state.ball)
                .push_player(&state.player1)
                .push_player(&state.player2)
                .build(&self.device);

            stg_vertex.copy_to_buffer(&mut encoder, &self.vertex_buffer);
            stg_index.copy_to_buffer(&mut encoder, &self.index_buffer);
            num_indices
        } else {
            0
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations::default(),
            }],
            depth_stencil_attachment: None,
        });

        if num_indices != 0 {
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_indexed(0..num_indices, 0, 0..1);
        }

        drop(render_pass);
        if state.title_text.visible {
            draw_text(&state.title_text, &mut self.glyph_brush);
        }
        if state.play_button.visible {
            draw_text(&state.play_button, &mut self.glyph_brush);
        }
        if state.quit_button.visible {
            draw_text(&state.quit_button, &mut self.glyph_brush);
        }
        if state.player1_score.visible {
            draw_text(&state.player1_score, &mut self.glyph_brush);
        }
        if state.player2_score.visible {
            draw_text(&state.player2_score, &mut self.glyph_brush);
        }
        if state.win_text.visible {
            draw_text(&state.win_text, &mut self.glyph_brush);
        }

        self.glyph_brush
            .draw_queued(
                &self.device,
                &mut self.staging_belt,
                &mut encoder,
                &frame.view,
                self.sc_desc.width,
                self.sc_desc.height,
            )
            .unwrap();

        self.staging_belt.finish();
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}

fn draw_text(text: &state::Text, glyph_brush: &mut wgpu_glyph::GlyphBrush<()>) {
    let layout = wgpu_glyph::Layout::default().h_align(if text.centered {
        wgpu_glyph::HorizontalAlign::Center
    } else {
        wgpu_glyph::HorizontalAlign::Left
    });

    let section =
        Section {
            screen_position: text.position.into(),
            bounds: text.bounds.into(),
            layout,
            ..Section::default()
        }
        .add_text(Text::new(&text.text).with_color(text.color).with_scale(
            if text.focused {
                text.size + 8.0
            } else {
                text.size
            },
        ));

    glyph_brush.queue(section);
}

fn create_render_pipeline(
    device: &Device,
    pipeline_layout: &PipelineLayout,
    color_format: TextureFormat,
    vertex_desc: &[VertexBufferDescriptor],
    vs_module: ShaderModule,
    fs_module: ShaderModule,
) -> RenderPipeline {
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Main Render Pipeline"),
        layout: Some(pipeline_layout),
        vertex_stage: ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: SHADER_ENTRY_POINT_NAME,
        },
        fragment_stage: Some(ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: SHADER_ENTRY_POINT_NAME,
        }),
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::Back,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        primitive_topology: PrimitiveTopology::TriangleList,
        color_states: &[ColorStateDescriptor {
            format: color_format,
            alpha_blend: BlendDescriptor::REPLACE,
            color_blend: BlendDescriptor::REPLACE,
            write_mask: ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: VertexStateDescriptor {
            index_format: IndexFormat::Uint32,
            vertex_buffers: vertex_desc,
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}
