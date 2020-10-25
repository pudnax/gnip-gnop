#![allow(clippy::single_match)]
pub(crate) use eyre::*;
use futures::executor::block_on;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod input;
mod math;
mod render;
mod state;
mod system;
mod util;

use input::Input;
use render::Renderer;
use system::System;

fn main() -> Result<()> {
    env_logger::init();
    color_eyre::install()?;

    let title = env!("CARGO_PKG_NAME");
    let event_loop = EventLoop::new();
    let monitor = event_loop
        .available_monitors()
        .next()
        .ok_or_else(|| eyre!("Failed to handle available monitor."))?;
    let video_mode = monitor
        .video_modes()
        .next()
        .ok_or_else(|| eyre!("Failed to obtain a video mode. Always panics in web."))?;
    let window = WindowBuilder::new()
        .with_visible(false)
        .with_title(title)
        .build(&event_loop)?;

    let mut renderer = block_on(Renderer::new(&window, &video_mode))?;

    let mut state = state::State {
        ball: state::Ball {
            position: (0.0, 0.0).into(),
            velocity: (0.0, 0.0).into(),
            radius: 0.05,
            visible: true,
        },
        player1: state::Player {
            position: (-0.8, 0.0).into(),
            size: (0.05, 0.4).into(),
            score: 0,
            visible: true,
        },
        player2: state::Player {
            position: (0.8, 0.0).into(),
            size: (0.05, 0.4).into(),
            score: 0,
            visible: true,
        },
        title_text: state::Text {
            position: (20.0, 20.0).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::from("PONG"),
            size: 64.0,
            ..Default::default()
        },
        play_button: state::Text {
            position: (40.0, 100.0).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::from("Play"),
            size: 32.0,
            centered: false,
            ..Default::default()
        },
        quit_button: state::Text {
            position: (40.0, 160.0).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::from("Quit"),
            size: 32.0,
            ..Default::default()
        },
        player1_score: state::Text {
            position: (renderer.width() * 0.25, 20.0).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::from("0"),
            size: 32.0,
            ..Default::default()
        },
        player2_score: state::Text {
            position: (renderer.width() * 0.75, 20.0).into(),
            color: (1.0, 1.0, 1.0, 1.0).into(),
            text: String::from("0"),
            size: 32.0,
            ..Default::default()
        },
        win_text: state::Text {
            position: (renderer.width() * 0.5, renderer.height() * 0.5).into(),
            bounds: (renderer.width(), state::UNBOUNDED_F32).into(),
            size: 32.0,
            centered: true,
            ..Default::default()
        },
        game_state: state::GameState::MainMenu,
        prev_state: state::GameState::Quiting,
    };

    let mut events = Vec::new();
    let mut input = Input::new();

    let mut menu_system = system::MenuSystem;
    let mut serving_system = system::ServingSystem::new();
    let mut play_system = system::PlaySystem;
    let ball_system = system::BallSystem;
    let mut game_over_system = system::GameOverSystem::new();
    let base_render_system = system::BaseSystem;

    let mut visiblity_system = system::VisibilitySystem;
    visiblity_system.start(&mut state);

    menu_system.start(&mut state);

    window.set_visible(true);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = if state.game_state == state::GameState::Quiting {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => state.game_state = state::GameState::Quiting,
                WindowEvent::KeyboardInput { input: w_input, .. } => match w_input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    } => {
                        state.game_state = state::GameState::Quiting;
                    }
                    KeyboardInput {
                        state: key_state,
                        virtual_keycode: Some(key),
                        ..
                    } => {
                        let input_handled = match state.game_state {
                            state::GameState::Quiting => true,
                            _ => {
                                let handled = input.update(key, key_state);
                                if key_state == &ElementState::Pressed {
                                    base_render_system.update_state(
                                        &input,
                                        &mut state,
                                        &mut events,
                                    );
                                }
                                handled
                            }
                        };
                        if !input_handled {
                            process_input(key_state, key, control_flow)
                        }
                    }
                    _ => {}
                },
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(*physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.resize(**new_inner_size);
                }
                _ => {}
            },
            Event::RedrawRequested(_) => {
                visiblity_system.update_state(&input, &mut state, &mut events);
                match state.game_state {
                    state::GameState::MainMenu => {
                        menu_system.update_state(&input, &mut state, &mut events);
                        if state.game_state == state::GameState::Serving {
                            serving_system.start(&mut state);
                        }
                    }
                    state::GameState::Serving => {
                        serving_system.update_state(&input, &mut state, &mut events);
                        play_system.update_state(&input, &mut state, &mut events);
                        if state.game_state == state::GameState::Playing {
                            play_system.start(&mut state);
                        }
                    }
                    state::GameState::Playing => {
                        ball_system.update_state(&input, &mut state, &mut events);
                        play_system.update_state(&input, &mut state, &mut events);
                        if state.game_state == state::GameState::Serving {
                            serving_system.start(&mut state);
                        } else if state.game_state == state::GameState::GameOver {
                            game_over_system.start(&mut state);
                        }
                    }
                    state::GameState::GameOver => {
                        game_over_system.update_state(&input, &mut state, &mut events);
                        if state.game_state == state::GameState::MainMenu {
                            menu_system.start(&mut state);
                        }
                    }
                    state::GameState::Quiting => {}
                    state::GameState::Base => {
                        // base_render_system.update_state(&input, &mut state, &mut events);
                        use state::GameState::*;
                        match state.game_state {
                            MainMenu => menu_system.start(&mut state),
                            Playing => play_system.start(&mut state),
                            Serving => serving_system.start(&mut state),
                            GameOver => game_over_system.start(&mut state),
                            Quiting | state::GameState::Base => {}
                        }
                    }
                }

                match renderer.render_state(&state) {
                    Ok(_) => {}
                    Err(e) => panic!("Panic requesting a renderer frame with an error:\n {}", e),
                };
                if state.game_state != state::GameState::Quiting {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    })
}

fn process_input(
    element_state: &ElementState,
    keycode: &VirtualKeyCode,
    control_flow: &mut ControlFlow,
) {
    match (keycode, element_state) {
        (VirtualKeyCode::Escape, ElementState::Pressed) => {
            *control_flow = ControlFlow::Exit;
        }
        _ => {}
    }
}
