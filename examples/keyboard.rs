use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let mut enigo = Enigo::new();

    // select all
    thread::sleep(Duration::from_secs(10));
    enigo.key_click(Key::Layout('a'));
    thread::sleep(Duration::from_secs(10));
    enigo.key_click(Key::Layout('a'));
}
