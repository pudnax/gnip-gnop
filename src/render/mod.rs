use crate::include_str_from_outdir;
use eyre::*;
use wgpu::{util::*, *};
use winit::{event::*, window::Window};

use crate::math::Vec2;

mod buffers;
use buffers::Vertex;

pub const SHADER_ENTRY_POINT_NAME: &str = "main";

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
    instance: Instance,
    surface: Surface,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    swap_chain: SwapChain,
    sc_desc: SwapChainDescriptor,
    render_pipeline: RenderPipeline,
    rp_layout: PipelineLayout,

    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_indices: u32,
}

impl Renderer {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
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
                None,
            )
            .await?;

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

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            usage: BufferUsage::VERTEX,
            contents: bytemuck::cast_slice(VERTEXES),
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            usage: BufferUsage::INDEX,
            contents: bytemuck::cast_slice(INDEXES),
        });
        let num_indices = INDEXES.len() as u32;

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

        Ok(Self {
            instance,
            surface,
            adapter,
            device,
            queue,
            swap_chain,
            sc_desc,
            render_pipeline,
            rp_layout,

            vertex_buffer,
            index_buffer,
            num_indices,
        })
    }

    pub fn render(&mut self) -> Result<()> {
        let frame = self.swap_chain.get_current_frame()?.output;

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
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..));
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    pub fn update(&mut self) {}

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }
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
