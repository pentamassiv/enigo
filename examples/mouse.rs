use enigo::{Enigo, MouseButton, MouseControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let wait_time = Duration::from_secs(2);

    let mut enigo = Enigo::new();
    // let mut enigo = enigo::XConnection::default();

    thread::sleep(Duration::from_secs(4));
    let (width, height) = enigo.main_display_size();
    println!("main_display_size: width: {width}, height: {height}");
}
