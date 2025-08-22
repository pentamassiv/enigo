use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings,
};
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::try_init().ok();
    thread::sleep(Duration::from_secs(2));
    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    enigo.key(Key::F11, Click).unwrap();

    /*
    enigo.key(Key::F11, Press).unwrap();
    enigo.key(Key::F11, Release).unwrap();
    // */
}
