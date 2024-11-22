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
    let mut enigo = enigo::Enigo::new(&Settings {
        windows_subject_to_mouse_speed_and_acceleration_level: true,
        ..Default::default()
    })
    .unwrap();

    enigo.move_mouse(0, 0, enigo::Coordinate::Abs).unwrap();

    for acceleration_level in 1..2 {
        for threshold1 in 6..7 {
            for threshold2 in 20..21 {
                enigo::set_mouse_thresholds_and_acceleration(
                    threshold1,
                    threshold2,
                    acceleration_level,
                )
                .unwrap();
                for &mouse_speed in [1, 10, 20].iter() {
                    enigo::set_mouse_speed(mouse_speed).unwrap();
                    for x in 0..30 {
                        for y in 0..30 {
                            enigo.move_mouse(x, y, enigo::Coordinate::Rel).unwrap();
                            let params_actual =
                                enigo::get_mouse_thresholds_and_acceleration().unwrap();
                            let mouse_speed_actual = enigo::get_mouse_speed().unwrap();
                            assert_eq!(params_actual, (threshold1, threshold2, acceleration_level));
                            assert_eq!(mouse_speed_actual, mouse_speed);
                            println!(
                                "({threshold1}, {threshold2}, {acceleration_level}, {mouse_speed}, ({x}, {y}), {:?}), ",
                                enigo.location().unwrap()
                            );

                            enigo.move_mouse(0, 0, enigo::Coordinate::Abs).unwrap();
                        }
                    }
                }
            }
        }
    }
    panic!();
}
