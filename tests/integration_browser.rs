use enigo::{
    Button,
    Coordinate::{Abs, Rel},
    Direction::{Click, Press, Release},
    Key, Keyboard, Mouse as _, Settings,
};

mod common;
use common::enigo_test::EnigoTest as Enigo;

#[test]
fn integration_browser_events() {
    let mut enigo = Enigo::new(&Settings::default());

    enigo.text("TestText❤️").unwrap(); // Fails on Windows (Message is empty???)
    enigo.key(Key::F1, Click).unwrap();
    enigo.key(Key::Control, Click).unwrap();
    enigo.key(Key::Backspace, Click).unwrap();
    enigo.key(Key::PageUp, Click).unwrap(); // Failing on Windows

    enigo.key(Key::Backspace, Press).unwrap();
    enigo.key(Key::Backspace, Release).unwrap();

    println!("Test mouse");
    enigo.move_mouse(100, 100, Abs).unwrap();
    enigo.move_mouse(200, 200, Abs).unwrap();
    enigo.move_mouse(20, 20, Rel).unwrap();
    enigo.move_mouse(-20, 20, Rel).unwrap();
    enigo.move_mouse(20, -20, Rel).unwrap();
    enigo.move_mouse(-20, -20, Rel).unwrap();
    enigo.button(Button::Left, Click).unwrap(); /*
                                                // let (x, y) = enigo.location().unwrap();
                                                // assert_eq!((200, 200), (x, y));
                                                // Relative moves fail on Windows
                                                // For some reason the values are wrong

                                                // enigo.scroll(1, Vertical).unwrap();
                                                // enigo.scroll(1, Horizontal).unwrap(); Fails on Windows
                                                enigo.main_display().unwrap();
                                                enigo.location().unwrap(); */
}

#[test]
#[cfg(target_os = "windows")]
// The relative mouse move is affected by mouse speed and acceleration level on
// Windows if the setting windows_subject_to_mouse_speed_and_acceleration_level
// is true
fn integration_browser_win_rel_mouse_move() {
    let mut enigo = Enigo::new(&Settings::default());
    let test_cases = vec![
        ((100, 100), Abs),
        ((0, 0), Rel),
        ((-0, 0), Rel),
        ((0, -0), Rel),
        ((-0, -0), Rel),
        ((1, 0), Rel),
        ((0, 1), Rel),
        ((-1, -1), Rel),
        ((4, 6), Rel),
        ((-42, 63), Rel),
        ((12, -20), Rel),
        ((-43, -1), Rel),
        ((200, 200), Rel),
        ((-200, 200), Rel),
        ((200, -200), Rel),
        ((-200, -200), Rel),
    ];
    for ((x, y), coord) in test_cases {
        enigo.move_mouse(x, y, coord).unwrap();
    }
}
