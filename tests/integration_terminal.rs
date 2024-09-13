use enigo::{Direction::Click, Enigo, Key, Keyboard, Settings};

#[test]
fn integration_terminal() {
    let text_to_type = vec!["hello", " world", "!"];
    check_text_in_terminal(text_to_type);
}

fn check_text_in_terminal(text_to_type: Vec<&str>) {
    env_logger::init();
    open_terminal();

    let mut enigo = Enigo::new(&Settings::default()).unwrap();

    // Start of the command to write to file
    enigo.text("echo \"").unwrap();

    // Type the actual text
    for text in &text_to_type {
        enigo.text(text).unwrap();
    }
    // End of the command
    enigo.text("\" > result.txt").unwrap();

    // Enter the command
    enigo.key(Key::Return, Click).unwrap();

    // Close terminal
    close_terminal(&mut enigo);

    // Read the file content into a String
    let file_content = std::fs::read_to_string("result.txt").expect("Unable to read the file");

    // Join the Vec<&str> into a single String
    let combined: String = text_to_type.concat();

    assert_eq!(file_content, combined, "The text was not correctly entered");
}

/// Open a terminal and maximize it
/// This uses GNOME Terminal on Linux
fn open_terminal() {
    use std::process::Command;

    // Execute commands based on the OS
    if cfg!(target_os = "linux") {
        // Run `gnome-terminal --maximize`
        Command::new("gnome-terminal")
            .arg("--maximize")
            .spawn()
            .expect("Failed to start gnome-terminal");
    } else if cfg!(target_os = "windows") {
        // Run `start /MAX cmd`
        Command::new("cmd")
            .args(&["/C", "start", "/MAX", "cmd"])
            .spawn()
            .expect("Failed to start cmd");
    } else if cfg!(target_os = "macos") {
        // Run AppleScript commands via `osascript`
        let script = r#"
                tell application "Terminal"
                    do script ""
                end tell
                tell application "System Events"
                    tell process "Terminal"
                        set frontmost to true
                        keystroke "f" using {command down, control down}
                    end tell
                end tell
            "#;

        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .spawn()
            .expect("Failed to run AppleScript");
    } else {
        panic!("Unsupported OS");
    }
    std::thread::sleep(std::time::Duration::from_secs(4));
}

/// Closes the terminal by typing "exit"
fn close_terminal(enigo: &mut Enigo) {
    enigo.text("exit").unwrap();
    enigo.key(Key::Return, Click).unwrap();
}
