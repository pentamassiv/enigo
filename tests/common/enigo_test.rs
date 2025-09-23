use std::net::{TcpListener, TcpStream};

use tungstenite::accept;

use enigo::{Axis, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings, agent::Token};

use super::browser_events::BrowserEvent;

// Number of minutes the test is allowed to run before timing out
// This is needed, because some of the websocket functions are blocking and
// would run indefinitely without a timeout if they don't receive a message
const TIMEOUT: u64 = 5;

pub struct EnigoTest {
    enigo: Enigo,
    websocket: tungstenite::WebSocket<TcpStream>,
    message_id: u32,
}

impl EnigoTest {
    pub fn new(settings: &Settings) -> Self {
        env_logger::try_init().ok();
        EnigoTest::start_timeout_thread();
        let enigo = Enigo::new(settings).unwrap();
        let _ = &*super::browser::BROWSER_INSTANCE; // Launch Firefox
        let websocket = Self::websocket();

        std::thread::sleep(std::time::Duration::from_secs(10)); // Give Firefox some time to launch
        Self {
            enigo,
            websocket,
            message_id: 0,
        }
    }

    fn websocket() -> tungstenite::WebSocket<TcpStream> {
        let listener = TcpListener::bind("127.0.0.1:26541").unwrap();
        println!("TcpListener was created");
        let (stream, addr) = listener.accept().expect("Unable to accept the connection");
        println!("New connection was made from {addr:?}");
        let websocket = accept(stream).expect("Unable to accept connections on the websocket");
        println!("WebSocket was successfully created");
        websocket
    }

    fn send_message(&mut self, msg: &str) {
        println!("Sending message: {msg}");
        self.websocket
            .send(tungstenite::Message::Text(tungstenite::Utf8Bytes::from(
                msg,
            )))
            .expect("Unable to send the message");
        println!("Sent message");
    }

    fn read_message(&mut self) -> BrowserEvent {
        println!("Waiting for message on Websocket");
        let message = self.websocket.read().unwrap();
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

    // Spawn a thread to handle the timeout
    fn start_timeout_thread() {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(TIMEOUT * 60));
            println!("Test suite exceeded the maximum allowed time of {TIMEOUT} minutes.");
            std::process::exit(1); // Exit with error code
        });
    }
}

impl Keyboard for EnigoTest {
    fn fast_text(&mut self, text: &str) -> enigo::InputResult<Option<()>> {
        let token = Token::Text(text.to_string());
        let msg = BrowserEvent::Syn(self.message_id, token);
        let msg_string = ron::to_string(&msg).expect("unable to serialize the message");
        self.send_message(&msg_string);
        self.message_id += 1;
        self.enigo.text(text).unwrap();
        assert_eq!(
            msg,
            self.read_message(),
            "Failed to simulate the text() function"
        );

        Ok(Some(()))
    }

    fn key(&mut self, key: Key, direction: Direction) -> enigo::InputResult<()> {
        let token = Token::Key(key, direction);
        let msg = BrowserEvent::Syn(self.message_id, token);
        let msg_string = ron::to_string(&msg).expect("unable to serialize the message");
        self.send_message(&msg_string);
        self.message_id += 1;
        self.enigo.key(key, direction).unwrap();
        assert_eq!(
            msg,
            self.read_message(),
            "Failed to simulate the key() function"
        );

        Ok(())
    }

    fn raw(&mut self, keycode: u16, direction: enigo::Direction) -> enigo::InputResult<()> {
        let token = Token::Raw(keycode, direction);
        let msg = BrowserEvent::Syn(self.message_id, token);
        let msg_string = ron::to_string(&msg).expect("unable to serialize the message");
        self.send_message(&msg_string);
        self.message_id += 1;
        self.enigo.raw(keycode, direction).unwrap();
        assert_eq!(
            msg,
            self.read_message(),
            "Failed to simulate the raw() function"
        );

        Ok(())
    }
}

impl Mouse for EnigoTest {
    fn button(&mut self, button: enigo::Button, direction: Direction) -> enigo::InputResult<()> {
        let token = Token::Button(button, direction);
        let msg = BrowserEvent::Syn(self.message_id, token);
        let msg_string = ron::to_string(&msg).expect("unable to serialize the message");
        self.send_message(&msg_string);
        self.message_id += 1;
        self.enigo.button(button, direction).unwrap();
        assert_eq!(
            msg,
            self.read_message(),
            "Failed to simulate the button() function"
        );

        Ok(())
    }

    // Edge cases don't work (mouse is at the left most border and can't move one to
    // the left)
    fn move_mouse(&mut self, x: i32, y: i32, coordinate: Coordinate) -> enigo::InputResult<()> {
        let token = Token::MoveMouse(x, y, coordinate);
        let msg = BrowserEvent::Syn(self.message_id, token);
        let msg_string = ron::to_string(&msg).expect("unable to serialize the message");
        self.send_message(&msg_string);
        self.message_id += 1;
        self.enigo.move_mouse(x, y, coordinate).unwrap();
        assert_eq!(
            msg,
            self.read_message(),
            "Failed to simulate the move_mouse() function"
        );

        Ok(())
    }

    fn scroll(&mut self, length: i32, axis: Axis) -> enigo::InputResult<()> {
        let token = Token::Scroll(length, axis);
        let msg = BrowserEvent::Syn(self.message_id, token);
        let msg_string = ron::to_string(&msg).expect("unable to serialize the message");
        self.send_message(&msg_string);
        self.message_id += 1;
        self.enigo.scroll(length, axis).unwrap();
        assert_eq!(
            msg,
            self.read_message(),
            "Failed to simulate the scroll() function"
        );

        Ok(())
    }

    fn main_display(&self) -> enigo::InputResult<(i32, i32)> {
        let enigo_res = self.enigo.main_display().unwrap();
        let rdev_res = rdev_main_display();
        assert_eq!(
            enigo_res, rdev_res,
            "enigo_res: {enigo_res:?}; rdev_res: {rdev_res:?}"
        );
        Ok(enigo_res)
    }

    fn location(&self) -> enigo::InputResult<(i32, i32)> {
        let enigo_res = self.enigo.location().unwrap();
        let mouse_position_res = mouse_position();
        assert_eq!(
            enigo_res, mouse_position_res,
            "enigo_res: {enigo_res:?}; rdev_res: {mouse_position_res:?}"
        );
        Ok(enigo_res)
    }
}

fn rdev_main_display() -> (i32, i32) {
    rdev::display_size()
        .map(|(x, y)| (x as i32, y as i32))
        .unwrap()
}

fn mouse_position() -> (i32, i32) {
    use mouse_position::mouse_position::Mouse;

    match Mouse::get_mouse_position() {
        Mouse::Position { x, y } => (x, y),
        _ => panic!("Unable to get the mouse position"),
    }
}
