use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    thread::sleep(Duration::from_secs(2));
    let mut enigo = Enigo::new();

    // select all
    enigo.key_click(Key::UpArrow);
    enigo.key_click(Key::LeftArrow);
}
