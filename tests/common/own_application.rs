use std::io::{Seek as _, SeekFrom, Write as _};

use enigo::{Axis, Coordinate, Direction, Key};
use tao::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

/// This application only displays a white window that is maximized and has no
/// header bar. It's only purpose is to register input and write it to the file
/// "event_log.txt". Each event that is written to the file is formatted so that
/// it can get deserialized as an enigo::agent::Token in order to simplify the
/// parsing. When the application gets started, it immediately writes it's width
/// and height to the first line. Since the application is maximized, we assume
/// this to be the size of the main display. This line is never deleted from the
/// file. Each integration test removes all lines from event_log.txt except the
/// very first line with the dimensions of the screen. Afterwards it simulates
/// input and then deserializes the events from the file to compare them with
/// the list of events it expected.
pub struct EnigoApp {}

impl EnigoApp {
    pub fn new(settings: &enigo::Settings) -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_decorations(false) // Disable the header bar
            .with_title("Test enigo")
            .build(&event_loop)
            .expect("unable to create the window");
        window.set_maximized(true);

        let log_file_path = "event_log.txt";
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // Clear the file content
            .open(log_file_path)
            .expect("unable to create the log file");

        // Get the window size (logical size)
        if let Some(main_display) = window.primary_monitor() {
            let tao::dpi::PhysicalSize { width, height } = main_display.size();
            println!("MainDisplay({width}, {height})");
            // writeln!(log_file, "MainDisplay({width}, {height})").unwrap();
        }

        event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                /*
                    WindowEvent::KeyboardInput { event, .. } => {
                    log_file.seek(SeekFrom::End(0)).unwrap();
                    writeln!(log_file, "KeyboardInput: {event:?}").unwrap();
                }
                */
                WindowEvent::CursorMoved { position, .. } => {
                    log_file.seek(SeekFrom::End(0)).unwrap();
                    writeln!(
                        log_file,
                        "MoveMouse({:?}, {:?}, Abs)",
                        position.x as i32, position.y as i32
                    )
                    .unwrap();
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    log_file.seek(SeekFrom::End(0)).unwrap();
                    writeln!(log_file, "Button({button:?}, {state:?})").unwrap();
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        tao::event::MouseScrollDelta::LineDelta(x, y) => {
                            if x.abs() <= 0.1 && y.abs() <= 0.1 {
                                // There was no scroll, so do nothing
                            } else if x.abs() <= 0.1 && y.abs() > 0.1 {
                                // Vertical scroll
                                log_file.seek(SeekFrom::End(0)).unwrap();
                                writeln!(log_file, "Scroll({:?}, Vertical)", -y).unwrap();
                            } else if x.abs() > 0.1 && y.abs() <= 0.1 {
                                // Horizontal scroll
                                log_file.seek(SeekFrom::End(0)).unwrap();
                                writeln!(log_file, "Scroll({:?}, Horizontal)", -x).unwrap();
                            } else {
                                // Scroll on both axis
                                panic!("scrolling on both axis is not yet supported")
                            }
                        }
                        tao::event::MouseScrollDelta::PixelDelta(_) => {
                            todo!("Enigo is currently unable to scroll by pixels")
                        }
                        _ => panic!("tao added a new variant"),
                    };
                }
                WindowEvent::ModifiersChanged(state) => {
                    log_file.seek(SeekFrom::End(0)).unwrap();
                    writeln!(log_file, "ModifierState({state:?})").unwrap()
                }
                WindowEvent::ReceivedImeText(string) => {
                    log_file.seek(SeekFrom::End(0)).unwrap();
                    writeln!(log_file, "ReceivedImeText({string:?})").unwrap()
                }
                _ => (),
            },
            Event::DeviceEvent { event, .. } => {
                log_file.seek(SeekFrom::End(0)).unwrap();
                writeln!(log_file, "DeviceEvent::{event:?}").unwrap()
            }
            _ => (),
        });
    }
}

impl enigo::Keyboard for EnigoApp {
    // This does not work for all text or the library does not work properly
    fn fast_text(&mut self, text: &str) -> enigo::InputResult<Option<()>> {}
    fn key(&mut self, key: Key, direction: Direction) -> enigo::InputResult<()> {}

    fn raw(&mut self, keycode: u16, direction: enigo::Direction) -> enigo::InputResult<()> {}
}

impl enigo::Mouse for EnigoApp {
    fn button(&mut self, button: enigo::Button, direction: Direction) -> enigo::InputResult<()> {}
    fn move_mouse(&mut self, x: i32, y: i32, coordinate: Coordinate) -> enigo::InputResult<()> {}
    fn scroll(&mut self, length: i32, axis: Axis) -> enigo::InputResult<()> {}
    fn main_display(&self) -> enigo::InputResult<(i32, i32)> {}
    fn location(&self) -> enigo::InputResult<(i32, i32)> {}
}
