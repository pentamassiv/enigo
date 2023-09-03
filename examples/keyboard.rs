use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let mut enigo = Enigo::new();

    // select all
    thread::sleep(Duration::from_secs(5));

    // select all
    enigo.key_click(Key::Layout('a'));
    enigo.key_click(Key::Layout('a'));
    enigo.key_click(Key::Layout('a'));
}
