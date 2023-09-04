use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let wait_time = Duration::from_secs(2);
    let mut enigo = Enigo::new();

    thread::sleep(Duration::from_secs(wait_time.as_secs()));

    //enigo.key_click(Key::PageDown);
    enigo.key_click(Key::UpArrow);
    enigo.key_click(Key::UpArrow);
    enigo.key_click(Key::LeftArrow);
    enigo.key_click(Key::LeftArrow);
    enigo.key_click(Key::DownArrow);
    enigo.key_click(Key::RightArrow);
    enigo.key_sequence("𝕊"); // Special char which needs two u16s to be encoded
}
