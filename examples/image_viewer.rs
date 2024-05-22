use std::sync::Arc;

use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, error::EventLoopError, event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

use egami::renderer::{self, FrameRenderContext, HasData, HasPosition, HasSize, Pair, WgpuFrameRenderContext, WgpuFrameRenderContextInit};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<renderer::WgpuFrameRenderContext>,
    frame_provider: Option<WgpuImageProvider>,
}

impl App {
    fn clear(&mut self) {
        self.window = None;
        self.render_context = None;
        self.frame_provider = None;
    }

    fn run() -> Result<(), EventLoopError> {
        env_logger::init();

        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = Self::default();
        event_loop.run_app(&mut app)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let attributes = Window::default_attributes()
            .with_title("xixi")
            .with_inner_size(PhysicalSize::new(2400, 960));

        let window = Arc::new(event_loop.create_window(attributes).unwrap());
        window.request_redraw();

        let window_size = window.inner_size();
        self.window = Some(Arc::clone(&window));
        self.frame_provider = Some(WgpuImageProvider::new());
        self.render_context = Some(WgpuFrameRenderContext::init(WgpuFrameRenderContextInit {
            surface_handle: window.into(),
            surface_size: (window_size.width, window_size.height),
            clear_color: None,
        }));
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.clear();
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.clear();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = &self.window {
            if window_id != window.id() {
                return;
            }

            let context = self.render_context.as_mut().expect("No renderer created!");

            match event {
                WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                    event: KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                    ..
                } => event_loop.exit(),
                WindowEvent::Resized(new_size) => context.configure((new_size.width, new_size.height)),
                WindowEvent::RedrawRequested => {
                    match context.draw_frame(self.frame_provider.as_ref().unwrap()) {
                        Ok(_) => {}
                        // Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprint!("Error: {}", e),
                    }
                    window.request_redraw();
                }
                _ => {},
            }
        }
    }
}

struct WgpuImageProvider {
    size: Pair<u32>,
    image_buffer: Vec<u8>,
}

struct WgpuImageFrame {
    size: Pair<u32>,
    buffer: Vec<u8>,
}

impl HasPosition<u32> for WgpuImageFrame {
    fn position(&self) -> Pair<u32> {
        (0, 0)
    }
}

impl HasSize<u32> for WgpuImageFrame {
    fn size(&self) -> Pair<u32> {
        self.size
    }
}

impl HasData for WgpuImageFrame {
    fn data(&self) -> &[u8] {
        &self.buffer
    }
}

impl WgpuImageProvider {
    fn new() -> Self {
        let bytes = include_bytes!("xixi.png");
        let image = image::load_from_memory(bytes).unwrap();

        let width = image.width();
        let height = image.height();
        let buffer = image.into_rgba8();
        let rgba8 = buffer.into_vec();

        Self {
            image_buffer: rgba8,
            size: (width, height),
        }
    }
}

impl<'iter> Iterator for &'iter WgpuImageProvider {
    type Item = WgpuImageFrame;

    fn next(&mut self) -> Option<Self::Item> {
        Some(WgpuImageFrame { size: self.size, buffer: self.image_buffer.clone() })
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    App::run()
}
