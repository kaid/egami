mod vertex;
mod renderer;

use std::sync::Arc;

use winit::{
    application::ApplicationHandler, error::EventLoopError, event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::Window
};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    renderer_state: Option<renderer::RendererState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window: Arc<Window> = Arc::new(event_loop.create_window(Window::default_attributes().with_title("xixi")).unwrap());
        window.request_redraw();

        self.window = Some(Arc::clone(&window));
        self.renderer_state = Some(renderer::RendererState::from(window));
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.renderer_state = None;
        self.window = None;
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
                let renderer_state = self.renderer_state.as_mut().expect("No renderer created!");

                if window_id == window.id() && !renderer_state.input(&event) {
                    match event {
                        WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                            ..
                        } => {
                            event_loop.exit();
                        },
                        WindowEvent::Resized(new_size) => renderer_state.resize(new_size),
                        WindowEvent::RedrawRequested => {
                            renderer_state.update();
                            match renderer_state.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => renderer_state.resize(renderer_state.size),
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

pub fn run() -> Result<(), EventLoopError> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app)
}