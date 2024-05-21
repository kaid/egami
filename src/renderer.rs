use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use std::borrow::{Borrow, BorrowMut};

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;
use wgpu::util::DeviceExt;
use winit::event::WindowEvent;

use crate::vertex;
use crate::vertex::Vertex;

trait FrameRenderContext: From<Self::Init> + Into<Self::Size> {
    type Size;
    type Position;
    type RenderError;
    type Init: Into<Self::Size>;
    type Frame: Into<Self::Size> + Into<Self::Position>;

    fn init(init: Self::Init) -> Self {
        let instance: Self = From::from(init);
        let size: Self::Size = init.into();
        instance.configure(size);
        instance
    }

    fn resize(&mut self, size: Self::Size) {
        self.configure(size);
    }

    fn configure(&self, size: Self::Size);

    fn draw_frame(&mut self, frame_provider: impl Iterator<Item = Self::Frame>) -> Result<(), Self::RenderError>;
}

#[derive(Debug)]
struct WgpuFrameRenderContext {
    queue: wgpu::Queue,
    device: wgpu::Device,
    clear_color: wgpu::Color,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,

    index_count: u32,
    index_buffer: wgpu::Buffer,
    vertex_buffer: Option<wgpu::Buffer>,

    texture: Option<wgpu::Texture>,
    bind_group: Option<wgpu::BindGroup>,
    render_pipeline: Option<wgpu::RenderPipeline>,
}

impl Into<PhysicalSize<u32>> for WgpuFrameRenderContext {
    fn into(self) -> PhysicalSize<u32> {
        PhysicalSize {
            width: self.config.width,
            height: self.config.height,
        }
    }
}

struct WgpuFrameRenderContextInit<'init> {
    surface_size: PhysicalSize<u32>,
    clear_color: Option<wgpu::Color>,
    surface_handle: wgpu::SurfaceTarget<'static>,

    indices: &'init [u16],
    vertices: &'init [Vertex],
}

impl Into<PhysicalSize<u32>> for WgpuFrameRenderContextInit<'_> {
    fn into(self) -> PhysicalSize<u32> {
        self.surface_size
    }
}

struct WgpuFrame {
    buffer: Vec<u8>,
    size: PhysicalSize<u32>,
    position: PhysicalPosition<u32>,
}

impl Into<PhysicalPosition<u32>> for WgpuFrame {
    fn into(self) -> PhysicalPosition<u32> {
        self.position
    }
}

impl Into<PhysicalSize<u32>> for WgpuFrame {
    fn into(self) -> PhysicalSize<u32> {
        self.size
    }
}

impl From<WgpuFrameRenderContextInit<'_>> for WgpuFrameRenderContext {
    fn from(WgpuFrameRenderContextInit {
        clear_color ,
        surface_size,
        surface_handle,

        indices,
        vertices,
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
            width: surface_size.width,
            height: surface_size.width,

            view_formats: vec![],
            format: surface_format,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            present_mode: surface_caps.present_modes[0],
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        };

        surface.configure(&device, &config);

        Self {
            queue,
            device,
            surface,
            config,
            clear_color: clear_color.unwrap_or(wgpu::Color::default()),

            index_count: indices.len() as u32,
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                usage: wgpu::BufferUsages::INDEX,
                contents: bytemuck::cast_slice(indices),
            }),
            vertex_buffer: None,

            texture: None,
            bind_group: None,
            render_pipeline: None,
        }
    }
}

impl FrameRenderContext for WgpuFrameRenderContext {
    type Frame = WgpuFrame;
    type Size = PhysicalSize<u32>;
    type Position = PhysicalPosition<u32>;
    type RenderError = wgpu::SurfaceError;
    type Init = WgpuFrameRenderContextInit<'_>;

    fn configure(&self, size: Self::Size) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn draw_frame(&mut self, mut frame_provider: impl Iterator<Item = Self::Frame>) -> Result<(), Self::RenderError> {
        match frame_provider.next() {
            None => Ok(()),
            Some(frame) => {
                match self.texture {
                    None => {
                        let frame_size: Self::Size = frame.into();

                        let texture_size = wgpu::Extent3d {
                            width: frame_size.width,
                            height: frame_size.height,
                            depth_or_array_layers: 1,
                        };

                        let texture_data_layout = wgpu::ImageDataLayout {
                            offset: 0,
                            rows_per_image: Some(frame.size.height),
                            bytes_per_row: Some(4 * frame_size.width),
                        };
            
                        self.texture = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                            label: Some("Image Texture"),
                            sample_count: 1,
                            view_formats: &[],
                            mip_level_count: 1,
                            size: texture_size,
                            dimension: wgpu::TextureDimension::D2,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        }));

                        self.queue.write_texture(
                            self.texture.unwrap().as_image_copy(),
                            &frame.buffer,
                            texture_data_layout,
                            texture_size,
                        );

                        let texture_view = self.texture.unwrap().create_view(&wgpu::TextureViewDescriptor::default());

                        let image_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                            label: Some("Image Sampler"),
                            address_mode_u: wgpu::AddressMode::Repeat,
                            address_mode_v: wgpu::AddressMode::Repeat,
                            address_mode_w: wgpu::AddressMode::Repeat,
                            mag_filter: wgpu::FilterMode::Linear,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        });

                        let bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

                        self.bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                        }));

                        let render_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("Render Pipeline Layout"),
                            bind_group_layouts: &[&bind_group_layout],
                            push_constant_ranges:&[],
                        });
                
                        self.render_pipeline = Some(self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                                    format: self.config.format,
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
                        }));

                    }
                    _ => (),
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

                    render_pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
                    render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
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

fn get_vertex_buffer(device: &wgpu::Device, ratios: (f32, f32)) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        usage: wgpu::BufferUsages::VERTEX,
        contents: bytemuck::cast_slice(&Vertex::from(ratios)),
    })
}

impl ImageProgramPayload {
    fn new(context: &WgpuFrameRenderContext, frame_dimensions: (u32, u32)) -> Self {
        let index_count = vertex::INDICES.len() as u32;

        let config = &context.config;
        let device = &context.device;

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

        let texture_view = image_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            index_count,
            index_buffer,
            vertex_buffer,
            render_pipeline,
            size: frame_dimensions,
            texture: Rc::new(image_texture),
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
    render_context: Rc<RefCell<WgpuFrameRenderContext>>,
    program_payload: Rc<RefCell<ImageProgramPayload>>,
}

impl ImageRenderer {
    pub fn from(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let ctx = Rc::new(RefCell::new(WgpuFrameRenderContext::init_from(window, (size.width, size.height), None)));
        let texture_provider = ImageProvider::new();
        let borrowed_ctx = ctx.as_ref().into_inner();
        let program_payload = Rc::new(RefCell::new(ImageProgramPayload::new(&borrowed_ctx, texture_provider.dimensions)));


        Self { render_context: ctx, program_payload }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        {
            let mut ctx = self.render_context.borrow_mut();
            if new_size.width > 0 && new_size.height > 0 && (new_size.height != ctx.config.height && new_size.width != ctx.config.width) {
                ctx.resize((new_size.width, new_size.height));
                let mut payload = self.program_payload.borrow_mut();
                let size = payload.size;
                let image_aspect_ratio = size.1 as f32 / size.0 as f32;
                payload.update_vertex_buffer(get_vertex_buffer(
                    &ctx.device,
                    (image_aspect_ratio, ctx.config.height as f32 / ctx.config.width as f32),
                ));
            }
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
        let mut context = self.render_context.borrow_mut();
        let texture_provider = &mut ImageProvider::new();
        let payload = self.program_payload.borrow();
        context.render(payload, texture_provider);
        Ok(())
    }
}
