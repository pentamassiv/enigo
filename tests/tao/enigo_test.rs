use std::io::{BufRead as _, Write as _};

use enigo::{
    agent::Token,
    Axis, Coordinate,
    Direction::{self, Click, Press, Release},
    Enigo, Key, Keyboard, Mouse, Settings,
};

const TIMEOUT: u64 = 5; // Number of minutes the test is allowed to run before timing out
                        // This is needed, because some of the websocket functions are blocking and
                        // would run indefinitely without a timeout if they don't receive a message

pub struct EnigoTest {
    main_display: (i32, i32),
    enigo: Option<Enigo>, // This has to be an Option so we can drop it within the Drop trait
    expected_tokens: Vec<Token>,
    mouse_location: (i32, i32),
}

impl EnigoTest {
    pub fn new(settings: &Settings) -> Self {
        env_logger::init();
        EnigoTest::start_timeout_thread();

        let mut enigo = Enigo::new(settings).unwrap();
        let executed_token = Vec::new();

        let mouse_start = (5, 43); // Arbitrary location that is not 0 and where x and y are unequal to detect bugs if e.g they are switched
                                   // It's best to select a y value >40 so that the mouse is not moved to the top bar of Gnome. Otherwise the tests fail since the top bar uses Wayland and not X11

        enigo
            .move_mouse(mouse_start.0, mouse_start.1, Coordinate::Abs)
            .expect("unable to move the mouse to the starting location");

        std::thread::sleep(std::time::Duration::from_secs(1));

        let log_file_path = "event_log.txt";
        let first_line = Self::read_first_line(log_file_path);
        Self::clear_log(log_file_path, &first_line);
        std::thread::sleep(std::time::Duration::from_secs(1));

        let main_display =
            if let Token::MainDisplay(width, height) = ron::from_str(&first_line).unwrap() {
                println!("Display size ({width}, {height})");
                (width, height)
            } else {
                panic!("not the size of the display")
            };

        Self {
            main_display,
            enigo: Some(enigo),
            expected_tokens: executed_token,
            mouse_location: mouse_start,
        }
    }

    fn start_timeout_thread() {
        // Spawn a thread to handle the timeout
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(TIMEOUT * 60));
            println!("Test suite exceeded the maximum allowed time of {TIMEOUT} minutes.");
            std::process::exit(1); // Exit with error code
        });
    }

    fn read_first_line(log_file_path: &str) -> String {
        // Read the first line from the file
        std::io::BufReader::new(
            std::fs::File::open(log_file_path).expect("unable to read log file"),
        )
        .lines()
        .next()
        .expect("unable to read the first line")
        .unwrap()
    }

    fn clear_log(log_file_path: &str, first_line: &str) {
        // Clear the log file and only write back the first line
        // This is needed because there can be other lines from previous tests
        let mut log_file = std::fs::OpenOptions::new()
            .create(false)
            .write(true)
            .truncate(true) // Clear the file content
            .open(log_file_path)
            .expect("unable to clear the log file and write the display dimensions to it");

        writeln!(log_file, "{first_line}").unwrap();
    }

    // ############## Mouse Trait #################
    // The following methods would be part of the Mouse trait,
    // but because the signature of the location and main_display methods
    // have to take a &mut self, they cannot be part of it

    pub fn button(
        &mut self,
        button: enigo::Button,
        direction: Direction,
    ) -> enigo::InputResult<()> {
        println!("button({button:?}, {direction:?})");
        if direction == Click {
            self.expected_tokens.push(Token::Button(button, Press));
            self.expected_tokens.push(Token::Button(button, Release));
        } else {
            self.expected_tokens.push(Token::Button(button, direction));
        }
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo.as_mut().unwrap().button(button, direction)
    }

    pub fn move_mouse(&mut self, x: i32, y: i32, coordinate: Coordinate) -> enigo::InputResult<()> {
        println!("move_mouse({x}, {y}, {coordinate:?})");

        // Calculate the expected resulting absolute coordinate of the mouse after the simulated input
        // The mouse will only be moved to the edges of the screen, so it must always be greater than 0 and lower or equal to the width/height of the screen
        let (expected_x, expected_y) = match coordinate {
            Coordinate::Abs => (x, y),
            Coordinate::Rel => (self.mouse_location.0 + x, self.mouse_location.1 + y),
        };
        let (expected_x, expected_y) = (std::cmp::max(0, expected_x), std::cmp::max(0, expected_y));
        let (expected_x, expected_y) = (
            std::cmp::min(self.main_display.0, expected_x), // TODO: Check if it should be self.main_display.0-1
            std::cmp::min(self.main_display.1, expected_y), // TODO: Check if it should be self.main_display.1-1
        );

        self.mouse_location = (expected_x, expected_y);
        self.expected_tokens
            .push(Token::MoveMouse(expected_x, expected_y, Coordinate::Abs));
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo.as_mut().unwrap().move_mouse(x, y, coordinate)
    }

    pub fn scroll(&mut self, length: i32, axis: Axis) -> enigo::InputResult<()> {
        println!("scroll({length}, {axis:?})");
        self.expected_tokens.push(Token::Scroll(length, axis));
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo.as_mut().unwrap().scroll(length, axis)
    }

    pub fn main_display(&mut self) -> enigo::InputResult<(i32, i32)> {
        println!("main_display()");
        // self.expected_tokens
        //     .push(Token::MainDisplay(self.main_display.0, self.main_display.1));
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo.as_mut().unwrap().main_display()
    }

    pub fn location(&mut self) -> enigo::InputResult<(i32, i32)> {
        println!("location()");
        // self.expected_tokens.push(Token::Location(
        //     self.mouse_location.0,
        //     self.mouse_location.1,
        // ));
        println!("expected_tokens:{:?}", self.expected_tokens);
        let res = self.enigo.as_mut().unwrap().location();
        assert_eq!(res.clone().unwrap(), self.mouse_location,);
        res
    }
}

impl Keyboard for EnigoTest {
    // This does not work for all text or the library does not work properly
    fn fast_text(&mut self, text: &str) -> enigo::InputResult<Option<()>> {
        println!("text({text})");
        self.expected_tokens.push(Token::Text(text.to_string()));
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo
            .as_mut()
            .unwrap()
            .text(text)
            .expect("Unable to enter text");
        Ok(Some(()))
    }

    fn key(&mut self, key: Key, direction: Direction) -> enigo::InputResult<()> {
        println!("key({key:?}, {direction:?})");
        if direction == Click {
            self.expected_tokens.push(Token::Key(key, Press));
            self.expected_tokens.push(Token::Key(key, Release));
        } else {
            self.expected_tokens.push(Token::Key(key, direction));
        }
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo.as_mut().unwrap().key(key, direction)
    }

    fn raw(&mut self, keycode: u16, direction: enigo::Direction) -> enigo::InputResult<()> {
        println!("raw({keycode}, {direction:?})");
        if direction == Click {
            self.expected_tokens.push(Token::Raw(keycode, Press));
            self.expected_tokens.push(Token::Raw(keycode, Release));
        } else {
            self.expected_tokens.push(Token::Raw(keycode, direction));
        }
        println!("expected_tokens:{:?}", self.expected_tokens);
        self.enigo.as_mut().unwrap().raw(keycode, direction)
    }
}

impl Drop for EnigoTest {
    fn drop(&mut self) {
        // On macOS, it's crucial to drop the `Enigo` struct only after all simulated events have been processed by the OS.
        // Dropping `Enigo` before this may result in the events being ignored.
        // To test proper event handling, we set self.enigo to none and drop the struct immediately after the last simulated event and verify that the event was processed correctly.
        self.enigo = None;

        std::thread::sleep(std::time::Duration::from_secs(2));

        // Compare the events received by the application with the list of expected events
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open("event_log.txt")
            .expect("unable to read the events from the log file");
        let reader = std::io::BufReader::new(&file);
        let actual_tokens: Vec<Token> = reader
            .lines()
            .skip(1) // Skip the first line with the dimensions of the screen
            .map(|l| l.unwrap())
            .map(|l| ron::from_str(&l).unwrap())
            .collect();

        assert_eq!(actual_tokens, self.expected_tokens);
    }
}
