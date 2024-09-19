use std::net::{TcpListener, TcpStream};

use tungstenite::accept;

use enigo::{
    Axis, Coordinate,
    Direction::{self, Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};

use super::browser_events::BrowserEvent;

const TIMEOUT: u64 = 5; // Number of minutes the test is allowed to run before timing out
                        // This is needed, because some of the websocket functions are blocking and
                        // would run indefinitely without a timeout if they don't receive a message

pub struct EnigoTest {
    enigo: Option<Enigo>, // This has to be an Option so we can drop it within the Drop trait before comparing the events
    display_size: (i32, i32),
    mouse: (i32, i32),
    websocket: tungstenite::WebSocket<TcpStream>,
    expected_events: Vec<BrowserEvent>,
}

impl EnigoTest {
    pub fn new(settings: &Settings) -> Self {
        env_logger::init();
        EnigoTest::start_timeout_thread();
        let mut enigo = Enigo::new(settings).unwrap();
        let start = (100, 100);
        enigo
            .move_mouse(start.0, start.1, Coordinate::Abs)
            .expect("unable to move the mouse to the starting position");
        std::thread::sleep(std::time::Duration::from_secs(1));

        let mouse_starting_position = enigo
            .location()
            .expect("unable to get the starting position of the mouse");
        assert_eq!(
            mouse_starting_position, start,
            "mouse is not at the expecting starting position"
        );
        let mouse = mouse_starting_position;

        // Create a listener for Firefox to connect to
        let listener = TcpListener::bind("127.0.0.1:26541").unwrap();
        println!("TcpListener was created");
        // Launch Firefox
        let _ = &*super::browser::BROWSER_INSTANCE;
        // Wait for a connection from the browser
        let (stream, addr) = listener.accept().expect("Unable to accept the connection");
        println!("New connection was made from {addr:?}");
        let mut websocket = accept(stream).expect("Unable to accept connections on the websocket");
        println!("WebSocket was successfully created");

        // Wait for Firefox to be ready
        let ev = Self::read_message(&mut websocket);
        let display_size = if let BrowserEvent::Ready(screen_width, screen_height) = ev {
            println!("Browser was opened and is ready to receive input");
            (screen_width, screen_height)
        } else {
            panic!("BrowserEvent was not Open: {ev:?}");
        };
        std::thread::sleep(std::time::Duration::from_secs(1));

        Self {
            enigo: Some(enigo),
            display_size,
            mouse,
            websocket,
            expected_events: Vec::new(),
        }
    }

    /// Send a message over the websocket to the browser
    fn send_message(&mut self, msg: &str) {
        println!("Sending message: {msg}");
        self.websocket
            .send(tungstenite::Message::Text(msg.to_string()))
            .expect("Unable to send the message");
        println!("Sent message");
    }

    /// Block until a message can be read from the websocket
    fn read_message(websocket: &mut tungstenite::WebSocket<TcpStream>) -> BrowserEvent {
        println!("Waiting for message on Websocket");
        let message = websocket.read().unwrap();
        println!("Processing message");

        let Ok(browser_event) = BrowserEvent::try_from(message) else {
            panic!("Other text received");
        };
        assert!(
            !(browser_event == BrowserEvent::Close),
            "Received a Close event"
        );
        browser_event
    }

    /// Spawn a thread to handle the timeout
    fn start_timeout_thread() {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(TIMEOUT * 60));
            println!("Test suite exceeded the maximum allowed time of {TIMEOUT} minutes.");
            std::process::exit(1); // Exit with error code
        });
    }

    /// Check if all currently expected events were actually received and removes them from the Vec
    fn check_events(&mut self) {
        for expected_event in self.expected_events.drain(..) {
            let actual_event = Self::read_message(&mut self.websocket);
            assert_eq!(expected_event, actual_event);
            println!("{:?} was actually received", expected_event);
        }
    }

    /// Get the name of the Key
    fn key_name(key: Key) -> String {
        if let Key::Unicode(char) = key {
            format!("{char}")
        } else {
            format!("{key:?}") //.to_lowercase()
        }
    }
}

impl Keyboard for EnigoTest {
    // This does not work for all text or the library does not work properly
    fn fast_text(&mut self, text: &str) -> enigo::InputResult<Option<()>> {
        // self.send_message("ClearText");
        //  println!("Attempt to clear the text");
        //  self.expected_events.push(BrowserEvent::ReadyForText); // Kinda pointless now that we no longer wait for it
        self.enigo
            .as_mut()
            .unwrap()
            .text(text)
            .expect("Unable to send text");
        println!("Executed enigo.text({text})");
        self.expected_events
            .push(BrowserEvent::Text(text.to_string()));
        self.send_message("GetText");

        Ok(None)
    }

    fn key(&mut self, key: Key, direction: Direction) -> enigo::InputResult<()> {
        self.enigo
            .as_mut()
            .unwrap()
            .key(key, direction)
            .expect("failed to enter the key");
        println!("Executed enigo.key({key:?}, {direction:?})");

        let key_name = EnigoTest::key_name(key);
        if matches!(direction, Click | Press) {
            self.expected_events
                .push(BrowserEvent::KeyDown(key_name.clone()));
        };
        if matches!(direction, Click | Release) {
            self.expected_events.push(BrowserEvent::KeyUp(key_name));
        };
        Ok(())
    }

    fn raw(&mut self, keycode: u16, direction: enigo::Direction) -> enigo::InputResult<()> {
        todo!()
    }
}

impl Mouse for EnigoTest {
    fn button(&mut self, button: enigo::Button, direction: Direction) -> enigo::InputResult<()> {
        self.enigo
            .as_mut()
            .unwrap()
            .button(button, direction)
            .expect("failed to press the button");
        println!("Executed enigo.button({button:?}, {direction:?})");

        if matches!(direction, Click | Press) {
            self.expected_events
                .push(BrowserEvent::MouseDown(button as u32));
        };
        if matches!(direction, Click | Release) {
            self.expected_events
                .push(BrowserEvent::MouseUp(button as u32));
        };
        Ok(())
    }

    fn move_mouse(&mut self, x: i32, y: i32, coordinate: Coordinate) -> enigo::InputResult<()> {
        use std::cmp::{max, min};

        let prev_coordinate = self.mouse;
        self.enigo
            .as_mut()
            .unwrap()
            .move_mouse(x, y, coordinate)
            .expect("unable to move the mouse");
        println!("Executed enigo.move_mouse({x}, {y}, {coordinate:?})");

        let next_coordinate = match coordinate {
            Coordinate::Abs => (x, y),
            Coordinate::Rel => (prev_coordinate.0 + x, prev_coordinate.0 + y),
        };
        // The mouse can never be moved to a negative coordinate
        let next_coordinate = (max(next_coordinate.0, 0), max(next_coordinate.1, 0));
        // The mouse can never be moved to a coordinate larger than the display size
        let next_coordinate = (
            min(next_coordinate.0, self.display_size.0),
            min(next_coordinate.1, self.display_size.1),
        );
        self.mouse = next_coordinate;
        self.expected_events.push(BrowserEvent::MouseMove(
            (
                next_coordinate.0 - prev_coordinate.0,
                next_coordinate.1 - prev_coordinate.1,
            ),
            next_coordinate,
        ));

        Ok(())
    }

    fn scroll(&mut self, length: i32, axis: Axis) -> enigo::InputResult<()> {
        self.enigo
            .as_mut()
            .unwrap()
            .scroll(length, axis)
            .expect("unable to scroll");
        println!("Executed enigo.scroll({length}, {axis:?})");

        // On some platforms it is not possible to scroll multiple lines so we
        // repeatedly scroll. In order for this test to work on all platforms,
        // we have to calculate the number of expected events and their values
        // The following values are taken from the Github runners
        // It is the number of events we have to create, the number of lines we can scroll with each event, the height of each line and the
        let (no_events, no_lines, line_height, scroll_width) = if cfg!(target_os = "windows") {
            (1, 1, 20, 114)
        } else if cfg!(target_os = "macos") {
            (1, 1, 20, 114)
        } else {
            (1, 1, 20, 114)
        };

        // The length is the number of lines that can be scrolled in one event times the height of each line
        let length = match axis {
            Axis::Horizontal => (no_lines * scroll_width, 0),
            Axis::Vertical => (0, no_lines * line_height),
        };

        for _ in 0..no_events {
            self.expected_events
                .push(BrowserEvent::MouseScroll(length.0, length.1));
        }

        Ok(())
    }

    fn main_display(&self) -> enigo::InputResult<(i32, i32)> {
        let res = self
            .enigo
            .as_ref()
            .unwrap()
            .main_display()
            .expect("can't get size of the display");
        println!("Executed enigo.main_display()");
        assert_eq!(res, self.display_size);

        Ok(res)
    }

    fn location(&self) -> enigo::InputResult<(i32, i32)> {
        let res = self
            .enigo
            .as_ref()
            .unwrap()
            .location()
            .expect("can't get the position of the mouse");
        println!("Executed enigo.location()");
        let expected_position = self.mouse;
        assert_eq!(
            res, expected_position,
            "position of the mouse is not what was expected"
        );

        Ok(res)
    }
}

impl Drop for EnigoTest {
    fn drop(&mut self) {
        // On macOS, it's crucial to drop the `Enigo` struct only after all simulated events have been processed by the OS.
        // Dropping `Enigo` before this may result in the events being ignored.
        // To test proper event handling, we set self.enigo to none and drop the struct immediately after the last simulated event and verify that the event was processed correctly.
        self.enigo = None;

        std::thread::sleep(std::time::Duration::from_secs(2));

        // Check if all expected events were received
        println!("Expected events: {:?}", self.expected_events);
        self.check_events();
    }
}
