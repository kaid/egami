use std::sync::Arc;

use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, error::EventLoopError, event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

use egami::renderer;

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    renderer: Option<renderer::Renderer>,
}

impl App {
    fn clear(&mut self) {
        self.renderer = None;
        self.window = None;
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

        let window: Arc<Window> = Arc::new(event_loop.create_window(attributes).unwrap());
        window.request_redraw();

        self.window = Some(Arc::clone(&window));
        self.renderer = Some(renderer::Renderer::from(window));
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
        match &self.window {
            None => return,
            Some(window) => {
                let renderer = self.renderer.as_mut().expect("No renderer created!");

                if window_id == window.id() && !renderer.input(&event) {
                    match event {
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                            ..
                        } => event_loop.exit(),
                        WindowEvent::Resized(new_size) => renderer.resize(new_size),
                        WindowEvent::RedrawRequested => {
                            renderer.update();
                            match renderer.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                                Err(e) => eprint!("Error: {}", e),
                            }
                            window.request_redraw();
                        }
                        _ => {},
                    }
                }
            },
        }

    }
}


fn main() -> Result<(), winit::error::EventLoopError> {
    App::run()
}
