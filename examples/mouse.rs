use enigo::{Enigo, MouseButton, MouseControllable};
use std::thread;
use std::time::Duration;

fn main() {
    let wait_time = Duration::from_secs(2);

    let mut enigo = Enigo::new();
    // let mut enigo = enigo::XConnection::default();

    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::Left);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::Middle);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::Right);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::ScrollUp);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::ScrollDown);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::ScrollLeft);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::ScrollRight);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::Back);
    println!(".");
    thread::sleep(Duration::from_secs(4));
    enigo.mouse_click(MouseButton::Forward);
    println!(".");
}
