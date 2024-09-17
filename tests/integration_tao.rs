use enigo::{
    Coordinate::{Abs, Rel},
    Direction::{Click, Press, Release},
    Key, Keyboard, Settings,
};

mod tao;
use tao::enigo_test::EnigoTest as Enigo;

#[test]
fn integration_tao() {
    let mut enigo = Enigo::new(&Settings::default());

    let _ = enigo.location().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    enigo.move_mouse(100, 100, Abs).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    let _ = enigo.location().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    enigo.move_mouse(100, -50, Rel).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    let _ = enigo.location().unwrap();
}
