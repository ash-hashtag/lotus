use log::{error, info};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};
mod state;
mod voxel;
use state::State;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();
    info!("Initiating...");
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Lotus")
        .build(&event_loop)
        .unwrap();
    let mut state = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(State::new(&window))
        .unwrap();

    event_loop
        .run(move |event, control_flow| {
            match event {
                Event::WindowEvent { window_id, event } => {
                    if window_id == state.window().id() {
                        if !state.input(&event) {
                            match event {
                                WindowEvent::CloseRequested => {
                                    info!("Exiting...");
                                    control_flow.exit();
                                }
                                WindowEvent::Resized(new_size) => {
                                    state.resize(new_size);
                                }
                                WindowEvent::ScaleFactorChanged {
                                    scale_factor,
                                    inner_size_writer,
                                } => {
                                    error!(
                                        "NOT IMPLEMENTED ScaleFactorChanged Scale Factor: {} InnerSize: {:?}",
                                        scale_factor, inner_size_writer
                                    );
                                },
                                WindowEvent::RedrawRequested => {
                                    state.update();
                                    match state.render() {
                                        Ok(_) => {},
                                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size()),
                                        Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                                        Err(e) => error!("{e}")
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                },
                _ => {}
            };
        })
        .unwrap();
}
