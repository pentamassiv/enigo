use enigo::{Enigo, Key, KeyboardControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let mut enigo = Enigo::new();

    // select all
    thread::sleep(Duration::from_secs(5));

    // write text
    enigo.key_sequence("Hello World! here is a lot of text  ❤️");
    enigo.key_sequence("💣💩🔥");

    thread::sleep(Duration::from_secs(5));
    // select all
    enigo.key_down(Key::Control);
    enigo.key_click(Key::Layout('a'));
    enigo.key_up(Key::Control);
}
