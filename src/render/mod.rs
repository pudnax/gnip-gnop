use crate::include_str_from_outdir;
pub(crate) use eyre::*;
use wgpu::*;

pub struct Renderer {
    instance: Instance,
    device: Device,
    adapter: Adapter,
    swap_chain: SwapChain,
    sc_desc: SwapChainDescriptor,
}

impl Renderer {
    pub async fn new(window: &minifb::Window, (width, height): (u32, u32)) -> Result<Self> {
        let backend_bit = BackendBit::PRIMARY;
        let instance = Instance::new(backend_bit);
        println!(
            "All available adapters that match {:?} backend bit: {:?}",
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
            width,
            height,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Main Pipeline Layout Descriptor"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader_compiler =
            shaderc::Compiler::new().ok_or_else(|| eyre!("Failed to create shader compiler."))?;
        let pipeline = {
            let vs_src = include_str_from_outdir!("/shaders/shader.vert");
            let fs_src = include_str_from_outdir!("/shaders/shader.frag");
            let vs_spirv = shader_compiler
                .compile_into_spirv(
                    vs_src,
                    shaderc::ShaderKind::Vertex,
                    "shader.vert",
                    "main",
                    None,
                )
                .unwrap();
            let fs_spirv = shader_compiler
                .compile_into_spirv(
                    fs_src,
                    shaderc::ShaderKind::Fragment,
                    "shader.frag",
                    "main",
                    None,
                )
                .unwrap();
            let vs_module =
                device.create_shader_module(wgpu::util::make_spirv(&vs_spirv.as_binary_u8()));
            let fs_module =
                device.create_shader_module(wgpu::util::make_spirv(&fs_spirv.as_binary_u8()));

            create_render_pipeline(
                &device,
                &pipeline_layout,
                sc_desc.format,
                vertex_desc,
                vs_module,
                fs_module,
            )
        };

        todo!()
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
    todo!()
}
