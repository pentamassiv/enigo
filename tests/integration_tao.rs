use enigo::{
    Coordinate::{Abs, Rel},
    Direction::{Click, Press, Release},
    Key, Keyboard as _, Mouse as _, Settings,
};

mod common;
use common::own_application::EnigoApp as Enigo;

#[test]
fn integration_tao() {
    let mut enigo = Enigo::new(&Settings::default());

    let _ = enigo.key(Key::Unicode('a'), Click).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(5));
}
