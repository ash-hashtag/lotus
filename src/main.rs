use std::time::Instant;

use clap::Parser;
use log::{error, info};
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
mod camera;
mod resources;
mod state;
mod voxel;
mod pipelines;
mod app_config;
mod ui;
mod ecs;
mod noise;
mod engine_state;
mod commands;
use state::State;


#[derive(Debug)]
enum CustomEvents {
    UserCommand(String),
}

fn run() {
    

    let event_loop = EventLoopBuilder::<CustomEvents>::with_user_event().build().unwrap();
    // let event_loop = EventLoop::<CustomEvents>::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("Lotus")
        .build(&event_loop)
        .unwrap();
    let proxy = event_loop.create_proxy();
    let mut state = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(State::new(&window, proxy))
        .unwrap();

    info!("State initialized");

    let mut last_render_time = Instant::now();
    event_loop
        .run(move |event, window_target| {
            // info!("EVent: {:?}", event);
            window_target.set_control_flow(ControlFlow::Poll);
            match event {
                Event::NewEvents(_) => state.window().request_redraw(),
                Event::DeviceEvent { device_id: _, event } => {
                  match event {
                        DeviceEvent::MouseMotion { delta } => state.camera_controller.process_mouse(delta.0, delta.1),
                        _ => {},
                    };
                },
                Event::WindowEvent { window_id, event } => {
                    if window_id == state.window().id() {
                        if !state.input(&event) {
                            match event {
                                WindowEvent::CloseRequested => {
                                    info!("Exiting...");
                                    window_target.exit();
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
                                    let now = Instant::now();
                                    let dt = now - last_render_time;
                                    last_render_time = now;
                                    state.update(dt);
                                    match state.render() {
                                        Ok(_) => {},
                                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size()),
                                        Err(wgpu::SurfaceError::OutOfMemory) => window_target.exit(),
                                        Err(e) => error!("{e}")
                                    }
                                },
                                _ => {}
                            }
                        } 
                    }
                },
                Event::UserEvent(event) => {
                    match event {
                        CustomEvents::UserCommand(cmd) => {
                            info!("User issued a command '{}'", cmd);
                        },
                    }
                    
                }
                _ => {}
            };
        })
        .unwrap();
}

fn main() {
    let _ = fast_log::init(fast_log::Config::new().console().level(log::LevelFilter::Info)).unwrap();
    info!("Initiating...");


    // let stdin = std::io::stdin();
    // let mut lines = stdin.lines();
    // while let Some(Ok(line)) = lines.next() {
    //     match commands::Command::try_parse_from(line.split(' ').skip_while(|x| x.is_empty())) {
    //         Ok(command) => info!("{:?}", command),
    //         Err(err) => error!("{err}\n{}", commands::Command::help_string()),
    //     };
        
    // }

    run();

}
