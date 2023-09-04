use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let wait_time = Duration::from_secs(2);
    let mut enigo = Enigo::new();

    thread::sleep(Duration::from_secs(wait_time.as_secs()));

    enigo.key_down(Key::Layout('a'));
    thread::sleep(Duration::from_secs(1));
    enigo.key_up(Key::Layout('a'));
}
