use enigo::{
    Button,
    Direction::{Click, Press, Release},
    Enigo, Mouse, Settings,
    {Axis::Horizontal, Axis::Vertical},
    {Coordinate::Abs, Coordinate::Rel},
};
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();
    let enigo = Enigo::new(&Settings::default()).unwrap();

    for _ in 0..1000 {
        thread::sleep(Duration::from_secs(1));
        println!("mouse location: {:?}", enigo.location().unwrap());
    }
}
