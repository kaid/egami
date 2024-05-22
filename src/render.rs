use wgpu::util::DeviceExt;
use crate::vertex::{self, INDICES, Vertex};
use crate::types::{Pair, FrameRenderContext, HasData, HasPosition, HasSize, HasRatio};

impl HasRatio for Pair<u32> {
    fn ratio(&self) -> f32 {
        self.0 as f32 / self.1 as f32
    }

    fn inverse_ratio(&self) -> f32 {
        self.1 as f32 / self.0 as f32
    }
}

#[derive(Debug)]
pub struct WgpuFrameRenderContext {
    queue: wgpu::Queue,
    device: wgpu::Device,
    clear_color: wgpu::Color,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,

    index_count: u32,
    index_buffer: wgpu::Buffer,

    // frame_size: Option<Pair<u32>>,
    // texture: Option<wgpu::Texture>,
    // bind_group: Option<wgpu::BindGroup>,
    // vertex_buffer: Option<wgpu::Buffer>,
    // render_pipeline: Option<wgpu::RenderPipeline>,

    resources: Option<WgpuFrameRenderContextResources>,
}

#[derive(Debug)]
struct WgpuFrameRenderContextResources {
    frame_size: Pair<u32>,
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
}

impl HasSize<u32> for WgpuFrameRenderContext {
    fn size(&self) -> Pair<u32> {
        (self.config.width, self.config.height)
    }
}

pub struct WgpuFrameRenderContextInit {
    pub surface_size: Pair<u32>,
    pub clear_color: Option<wgpu::Color>,
    pub surface_handle: wgpu::SurfaceTarget<'static>,
}

impl HasSize<u32> for WgpuFrameRenderContextInit {
    fn size(&self) -> Pair<u32> {
        self.surface_size
    }
}

impl From<WgpuFrameRenderContextInit> for WgpuFrameRenderContext {
    fn from(WgpuFrameRenderContextInit {
        clear_color ,
        surface_size,
        surface_handle,
    }: WgpuFrameRenderContextInit) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(surface_handle).unwrap();

        let ((device, queue), adapter) = smol::block_on(async {
            let adapter = instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::default(),
            }).await.unwrap();

            (adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_limits: wgpu::Limits::default(),
                    required_features: wgpu::Features::empty(),
                },
                None,
            ).await.unwrap(), adapter)
        });

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            width: surface_size.0,
            height: surface_size.1,

            view_formats: vec![],
            format: surface_format,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            present_mode: surface_caps.present_modes[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        surface.configure(&device, &config);

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(INDICES),
        });

        Self {
            queue,
            config,
            device,
            surface,
            clear_color: clear_color.unwrap_or(wgpu::Color::default()),

            index_buffer,
            index_count: INDICES.len() as u32,

            // texture: None,
            // bind_group: None,
            // frame_size: None,
            // vertex_buffer: None,
            // render_pipeline: None,
            resources: None,
        }
    }
}

fn get_vertices(device: &wgpu::Device, frame_size: Pair<u32>, surface_size: Pair<u32>) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        usage: wgpu::BufferUsages::VERTEX,
        contents: bytemuck::cast_slice(&Vertex::get_vertices((frame_size.inverse_ratio(), surface_size.inverse_ratio()))),
    })
}

impl WgpuFrameRenderContextResources {
    fn new(config: &wgpu::SurfaceConfiguration, device: &wgpu::Device, frame_size: Pair<u32>, surface_size: Pair<u32>) -> Self {
        let texture_size = wgpu::Extent3d {
            width: frame_size.0,
            height: frame_size.1,
            depth_or_array_layers: 1,
        };

        let vertex_buffer = get_vertices(device, frame_size, surface_size);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Texture"),
            sample_count: 1,
            view_formats: &[],
            mip_level_count: 1,
            size: texture_size,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });
        
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Image Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Image Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&image_sampler),
                },
            ],
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges:&[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex::Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            texture,
            bind_group,
            frame_size,
            vertex_buffer,
            render_pipeline,
        }
    }

    fn queue_write_texture<Frame>(&self, queue: &wgpu::Queue, frame: &Frame)
    where
        Frame: HasSize<u32> + HasPosition<u32> + HasData
    {
        queue.write_texture(
            self.texture.as_image_copy(),
            &frame.data(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * frame.size().0),
                rows_per_image: Some(frame.size().1),
            },
            self.texture.size(),
        );
    }
}

impl FrameRenderContext for WgpuFrameRenderContext {
    type RenderError = wgpu::SurfaceError;
    type Init = WgpuFrameRenderContextInit;

    fn configure(&mut self, size: Pair<u32>) {
        self.config.width = size.0;
        self.config.height = size.1;
        self.surface.configure(&self.device, &self.config);

        match self.resources.as_mut() {
            Some(resources) => {
                resources.vertex_buffer = get_vertices(&self.device, resources.frame_size, size);
            },
            _ => (),
        }
    }

    fn draw_frame<Frame>(&mut self, mut frame_provider: impl Iterator<Item = Frame>) -> Result<(), Self::RenderError>
    where
        Frame: HasSize<u32> + HasPosition<u32> + HasData
    {
        match frame_provider.next() {
            None => Ok(()),
            Some(frame) => {
                let frame_size = frame.size();

                match self.resources.as_ref() {
                    Some(resources) => {
                        resources.queue_write_texture(&self.queue, &frame)
                    }
                    None => {
                        self.resources = Some(WgpuFrameRenderContextResources::new(&self.config, &self.device, frame_size, self.size()));
                        self.resources.as_mut().unwrap().queue_write_texture(&self.queue, &frame)
                    }
                }

                let output = self.surface.get_current_texture()?;
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });
    
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.clear_color),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        depth_stencil_attachment: None,
                    });

                    let resources = self.resources.as_ref().unwrap();

                    render_pass.set_pipeline(&resources.render_pipeline);
                    render_pass.set_bind_group(0, &resources.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                }

                self.queue.submit(std::iter::once(encoder.finish()));
                output.present();

                Ok(())
            }
        }

    }
}
