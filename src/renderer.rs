use std::sync::Arc;

use winit::event::WindowEvent;
use winit::window::Window;
use wgpu::util::DeviceExt;

use crate::vertex;
use crate::vertex::Vertex;

trait GraphicsContext {
    type Color;
    type Handle;
    type RenderError;
    type ProgramPayload;

    fn init_from(handle: impl Into<Self::Handle>, size: (u32, u32), clear_color: Option<Self::Color>) -> Self;

    fn render<Iter: Iterator<Item = Vec<u8>>>(
        &mut self,
        queue_state: impl Into<Self::ProgramPayload>,
        texture_provider: &mut Iter,
    ) -> Result<(), Self::RenderError>;
}

#[derive(Debug)]
struct WgpuRenderContext {
    size: (u32, u32),
    queue: wgpu::Queue,
    device: wgpu::Device,
    clear_color: wgpu::Color,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

// index_count, vertex_buffer, index_buffer, pipeline, bind_group, image_copy_texture, image_data_layout, texture_size
type WgpuProgramPayload = (
    u32,
    wgpu::Buffer,
    wgpu::Buffer,
    wgpu::RenderPipeline,
    wgpu::BindGroup,
    Arc<wgpu::Texture>,
    wgpu::ImageDataLayout,
    wgpu::Extent3d,
);

impl GraphicsContext for WgpuRenderContext {
    type Color = wgpu::Color;
    type RenderError = wgpu::SurfaceError;
    type Handle = wgpu::SurfaceTarget<'static>;
    type ProgramPayload = WgpuProgramPayload;

    fn init_from(handle: impl Into<Self::Handle>, size: (u32, u32), clear_color: Option<Self::Color>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(handle).unwrap();

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
            width: size.0,
            height: size.1,
            view_formats: vec![],
            format: surface_format,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            present_mode: surface_caps.present_modes[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        surface.configure(&device, &config);

        Self { size, queue, device, surface, config, clear_color: clear_color.unwrap_or(wgpu::Color::default()) }
    }
    
    fn render<Iter: Iterator<Item = Vec<u8>>>(
        &mut self,
        queue_state: impl Into<Self::ProgramPayload>,
        texture_provider: &mut Iter,
    ) -> Result<(), Self::RenderError> {
        let (
            index_count,
            vertex_buffer,
            index_buffer,
            render_pipeline,
            bind_group,
            texture,
            texture_data_layout,
            texture_size,
        ) = queue_state.into();

        if let Some(data) = texture_provider.next() {
            self.queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data,
                texture_data_layout,
                texture_size,
            );
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

            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..index_count, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Debug)]
struct ImageProgramPayload {
    size: (u32, u32),
    index_count: u32,
    index_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    texture: Arc<wgpu::Texture>,
    render_pipeline: wgpu::RenderPipeline,
}

impl Into<WgpuProgramPayload> for ImageProgramPayload {
    fn into(self) -> WgpuProgramPayload {
        // index_count, vertex_buffer, index_buffer, pipeline, bind_group, image_copy_texture, image_data_layout, texture_size
        let texture_size = wgpu::Extent3d {
            width: self.size.0,
            height: self.size.1,
            depth_or_array_layers: 1,
        };

        let texture_data_layout = wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * self.size.0),
            rows_per_image: Some(self.size.1),
        };

        (
            self.index_count,
            self.vertex_buffer,
            self.index_buffer,
            self.render_pipeline,
            self.bind_group,
            self.texture,
            texture_data_layout,
            texture_size,
        )
    }
}

fn get_vertex_buffer(device: &wgpu::Device, ratios: (f32, f32)) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        usage: wgpu::BufferUsages::VERTEX,
        contents: bytemuck::cast_slice(&Vertex::from(ratios)),
    })
}

impl ImageProgramPayload {
    fn new(context: WgpuRenderContext, frame_dimensions: (u32, u32)) -> Self {
        let index_count = vertex::INDICES.len() as u32;

        let WgpuRenderContext { config, device, .. } = context;

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(vertex::INDICES),
        });

        let (frame_width, frame_height) = frame_dimensions;
        let frame_aspect_ratio = frame_height as f32 / frame_width as f32;
        let vertex_buffer = get_vertex_buffer(&device, (frame_aspect_ratio, config.height as f32 / config.width as f32));

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let texture_size = wgpu::Extent3d {
            width: frame_width,
            height: frame_height,
            depth_or_array_layers: 1,
        };

        let image_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Image Texture"),
            sample_count: 1,
            view_formats: &[],
            mip_level_count: 1,
            size: texture_size,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        let texture_view = image_texture.create_view(&wgpu::TextureViewDescriptor::default());
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

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let image_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Image Bind Group"),
            layout: &texture_bind_group_layout,
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
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges:&[],
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
            index_count,
            index_buffer,
            vertex_buffer,
            render_pipeline,
            size: frame_dimensions,
            texture: Arc::new(image_texture),
            bind_group: image_bind_group,
        }
    }
}

struct ImageProvider {
    dimensions: (u32, u32),
    image_buffer: Vec<u8>,
}

impl ImageProvider {
    fn new() -> Self {
        let bytes = include_bytes!("xixi.png");
        let image = image::load_from_memory(bytes).unwrap();

        let width = image.width();
        let height = image.height();
        let buffer = image.into_rgba8();
        let rgba8 = buffer.into_vec();

        Self {
            dimensions: (width, height),
            image_buffer: rgba8,
        }
    }
}

impl Iterator for ImageProvider {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.image_buffer.clone())
    }
}

#[derive(Debug)]
pub struct ImageRenderer {
    program_payload: ImageProgramPayload,
    render_context: WgpuRenderContext,
}

impl ImageRenderer {
    pub fn from(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let render_context = WgpuRenderContext::init_from(window, (size.width, size.height), None);
        let texture_provider = ImageProvider::new();
        let program_payload = ImageProgramPayload::new(render_context, texture_provider.dimensions);


        Self { render_context, program_payload }
    }

    fn reset_vertex_buffer(&mut self) {
        let WgpuRenderContext { config, device, .. } = &mut self.render_context;
        let ImageProgramPayload { size, .. } = self.program_payload;
        let image_aspect_ratio = size.1 as f32 / size.0 as f32;
        self.program_payload.vertex_buffer = get_vertex_buffer(&device, (image_aspect_ratio, config.height as f32 / config.width as f32));
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let WgpuRenderContext { config, .. } = &mut self.render_context;
        if new_size.width > 0 && new_size.height > 0 && (new_size.height != config.height && new_size.width != config.width) {
            self.render_context.config.width = new_size.width;
            self.render_context.config.height = new_size.height;
            self.render_context.surface.configure(&self.render_context.device, &self.render_context.config);
            self.reset_vertex_buffer();
        }

        let _ = self.render();
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { .. } => {
                // let PhysicalPosition { x, y } = position;
                // let PhysicalSize { width, height } = self.size;

                // let r = x / width as f64;
                // let g = y / height as f64;
                // let b = (x + y) / (height + width) as f64;

                // self.color = wgpu::Color { r, g, b, a: 1.0 };

                true
            }
            _ => false
        }
    }

    pub fn update(&mut self) {
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        Ok(())
    }
}
