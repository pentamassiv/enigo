use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let wait_time = Duration::from_secs(2);
    let mut enigo = Enigo::new();

    thread::sleep(Duration::from_secs(wait_time.as_secs()));

    // select all
    enigo.key_down(Key::Control);
    enigo.key_click(Key::Layout('a'));
    enigo.key_up(Key::Control);
}
