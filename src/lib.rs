mod vertex;
mod renderer;

use winit::{
    error::EventLoopError, event::*, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::WindowBuilder
};

pub fn run() -> Result<(), EventLoopError> {
    env_logger::init();

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("xixixi")
        .build(&event_loop).unwrap();

    let mut state = renderer::RendererState::new(&window);

    event_loop.run(move |event, window_target| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window.id() && !state.input(event) => match event {
            WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            } => window_target.exit(),
            WindowEvent::Resized(new_size) => state.resize(*new_size),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => window_target.exit(),
                    Err(e) => eprint!("Error: {}", e),
                }
                state.window.request_redraw();
            }
            _ => {},
        }
        _ => {}
    })
}