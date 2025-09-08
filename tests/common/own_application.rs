use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, agent::Token};
use tao::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    keyboard::ModifiersState,
    platform::{run_return::EventLoopExtRunReturn as _, unix::EventLoopBuilderExtUnix as _},
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
pub struct EnigoApp {
    event_loop: EventLoop<()>,
    enigo: Enigo,
    modifier_state: ModifiersState,
}

impl EnigoApp {
    pub fn new(settings: &enigo::Settings) -> Self {
        let event_loop = EventLoopBuilder::new().with_any_thread(true).build();
        let window = WindowBuilder::new()
            .with_decorations(false) // Disable the header bar
            .with_title("Test enigo")
            .build(&event_loop)
            .expect("unable to create the window");
        window.set_maximized(true);

        // Get the window size (logical size)
        if let Some(main_display) = window.primary_monitor() {
            let tao::dpi::PhysicalSize { width, height } = main_display.size();
            println!("MainDisplay({width}, {height})");
        }

        let enigo = Enigo::new(settings).expect("Failed to establish enigo connection");

        Self {
            event_loop,
            enigo,
            modifier_state: ModifiersState::empty(),
        }
    }

    fn pump_till(&mut self, expected_token: Token) {
        self.event_loop.run_return(|event, _, _| {
            println!();
            println!("Processing event: {event:?}");
            let token = match event {
                Event::WindowEvent { event, .. } => try_from(event, self.modifier_state),
                Event::DeviceEvent {
                    device_id, event, ..
                } => todo!(),
                Event::NewEvents(_)
                | Event::UserEvent(_)
                | Event::Suspended
                | Event::Resumed
                | Event::MainEventsCleared
                | Event::RedrawRequested(_)
                | Event::RedrawEventsCleared
                | Event::LoopDestroyed
                | Event::Opened { .. }
                | Event::Reopen { .. } => return,
                _ => todo!(),
            };
            if let Some(token) = token {
                assert_eq!(expected_token, token);
            }
        });
    }
}

impl enigo::Keyboard for EnigoApp {
    // This does not work for all text or the library does not work properly
    fn fast_text(&mut self, text: &str) -> enigo::InputResult<Option<()>> {
        let res = self.enigo.fast_text(text);
        if res.is_ok() {
            self.pump_till(Token::Text(text.to_string()));
        }
        res
    }
    fn key(&mut self, key: Key, direction: Direction) -> enigo::InputResult<()> {
        let res = self.enigo.key(key, direction);
        if res.is_ok() {
            self.pump_till(Token::Key(key, direction));
        }
        res
    }
    fn raw(&mut self, keycode: u16, direction: enigo::Direction) -> enigo::InputResult<()> {
        let res = self.enigo.raw(keycode, direction);
        if res.is_ok() {
            self.pump_till(Token::Raw(keycode, direction));
        }
        res
    }
}

impl enigo::Mouse for EnigoApp {
    fn button(&mut self, button: enigo::Button, direction: Direction) -> enigo::InputResult<()> {
        let res = self.enigo.button(button, direction);
        if res.is_ok() {
            self.pump_till(Token::Button(button, direction));
        }
        res
    }
    fn move_mouse(&mut self, x: i32, y: i32, coordinate: Coordinate) -> enigo::InputResult<()> {
        let res = self.enigo.move_mouse(x, y, coordinate);
        if res.is_ok() {
            self.pump_till(Token::MoveMouse(x, y, coordinate));
        }
        res
    }
    fn scroll(&mut self, length: i32, axis: Axis) -> enigo::InputResult<()> {
        let res = self.enigo.scroll(length, axis);
        if res.is_ok() {
            self.pump_till(Token::Scroll(length, axis));
        }
        res
    }
    fn main_display(&self) -> enigo::InputResult<(i32, i32)> {
        self.enigo.main_display()
    }
    fn location(&self) -> enigo::InputResult<(i32, i32)> {
        self.enigo.location()
    }
}

fn try_from(event: WindowEvent, before_modifier_state: ModifiersState) -> Option<Token> {
    match event {
        WindowEvent::CloseRequested => {
            panic!("close requested. Impossible in the test");
        }
        WindowEvent::CursorMoved { position, .. } => {
            println!("MoveMouse({}, {}, Abs)", position.x, position.y);
            Some(Token::MoveMouse(
                position.x as i32,
                position.y as i32,
                Coordinate::Abs,
            ))
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let direction = from_state(state);
            let button = from_mouse_button(button);
            println!("Button({button:?}, {direction:?})");
            Some(Token::Button(button, direction))
        }
        WindowEvent::MouseWheel { delta, .. } => {
            let (x, y) = match delta {
                tao::event::MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
                tao::event::MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => (x, y),
                _ => unimplemented!("tao added a new variant"),
            };

            let length;
            let axis;
            if x.abs() <= 0.1 && y.abs() <= 0.1 {
                // There was no scroll, so do nothing
                return None;
            } else if x.abs() <= 0.1 && y.abs() > 0.1 {
                // Vertical scroll
                length = -y;
                axis = Axis::Vertical;
            } else if x.abs() > 0.1 && y.abs() <= 0.1 {
                // Horizontal scroll
                length = -x;
                axis = Axis::Horizontal;
            } else {
                // Scroll on both axis
                panic!("scrolling on both axis is not yet supported")
            };

            match delta {
                tao::event::MouseScrollDelta::LineDelta(_, _) => {
                    println!("Scroll({length}, {axis:?})");
                    Some(Token::Scroll(length as i32, axis))
                }
                tao::event::MouseScrollDelta::PixelDelta(_) => {
                    #[cfg(all(feature = "platform_specific", target_os = "macos"))]
                    {
                        println!("Scroll({length}, {axis:?})");
                        Some(Token::SmoothScroll(length as i32, axis))
                    }
                    #[cfg(not(all(feature = "platform_specific", target_os = "macos")))]
                    {
                        panic!("Smooth scrolling is not implemented on this platform")
                    }
                }
                _ => unreachable!("would have paniced in the previous match statement"),
            }
        }
        WindowEvent::ModifiersChanged(after_modifier_state) => {
            let pressed = after_modifier_state - before_modifier_state;
            let released = before_modifier_state - after_modifier_state;

            let (key, direction) = match (pressed.is_empty(), released.is_empty()) {
                (false, true) => (from_modifier_state(pressed), Direction::Press),
                (true, false) => (from_modifier_state(released), Direction::Release),
                _ => panic!("expected exactly one modifier to change"),
            };

            println!("Key({key:?}, {direction:?})");
            Some(Token::Key(key, direction))
        }
        WindowEvent::ReceivedImeText(string) => {
            println!("Text({string})");
            Some(Token::Text(string))
        }
        WindowEvent::KeyboardInput {
            device_id: _,
            event,
            is_synthetic: _,
            ..
        } => todo!(),
        // Not (yet) relevant events
        WindowEvent::Touch(_) => None,
        WindowEvent::AxisMotion { .. } => None,
        WindowEvent::TouchpadPressure { .. } => None,
        // Irrelevant events
        WindowEvent::Resized(_)
        | WindowEvent::Moved(_)
        | WindowEvent::Destroyed
        | WindowEvent::DroppedFile(_)
        | WindowEvent::HoveredFile(_)
        | WindowEvent::HoveredFileCancelled
        | WindowEvent::Focused(_)
        | WindowEvent::CursorEntered { .. }
        | WindowEvent::CursorLeft { .. }
        | WindowEvent::ScaleFactorChanged { .. }
        | WindowEvent::ThemeChanged(_)
        | WindowEvent::DecorationsClick => None,
        _ => panic!("Unknown WindowEvent"),
    }
}

fn from_state(state: ElementState) -> Direction {
    match state {
        ElementState::Pressed => Direction::Press,
        ElementState::Released => Direction::Release,
        _ => unreachable!(), /* ElementState only has two variants but for some reason is marked
                              * as non-exhaustive */
    }
}

fn from_mouse_button(button: MouseButton) -> enigo::Button {
    match button {
        MouseButton::Left => Button::Left,
        MouseButton::Right => Button::Right,
        MouseButton::Middle => Button::Middle,
        MouseButton::Other(0) => unimplemented!(), /* TODO: Find out the correct mapping for the */
        // other values
        MouseButton::Other(1) => unimplemented!(),
        MouseButton::Other(2) => unimplemented!(),
        MouseButton::Other(3) => unimplemented!(),
        MouseButton::Other(4) => unimplemented!(),
        _ => unreachable!(), /* MouseButton only has two variants but for some reason is marked
                              * as non-exhaustive */
    }
}

fn from_modifier_state(changed_modifier: ModifiersState) -> Key {
    match changed_modifier {
        ModifiersState::SHIFT => Key::Shift,
        ModifiersState::CONTROL => Key::Control,
        ModifiersState::ALT => Key::Alt,
        ModifiersState::SUPER => Key::Meta,
        other => panic!("unknown modifier {other:?}"),
    }
}
