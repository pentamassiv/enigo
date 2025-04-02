use std::{fs::File, io::Write};

use xkbcommon::xkb::{CONTEXT_NO_FLAGS, Context, FORMAT_TEXT_V1, KEYMAP_COMPILE_NO_FLAGS, Keymap};

fn main() {
    let content = std::fs::read_to_string("binary_keymap.txt").unwrap();

    // Split the content by commas, trim whitespace, parse to u8, and convert to a
    // char.
    let decoded: String = content
        .split(',')
        .filter_map(|s| s.trim().parse::<u8>().ok().map(char::from))
        .collect();

    let mut write_file = std::fs::File::create("./binary_keymap_decoded.txt").unwrap();
    write!(write_file, "{}", decoded).unwrap();
    // println!("{}", decoded);

    let received_keymap = std::fs::read_to_string("received_keymap.txt").unwrap();
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;
    let keymap =
        Keymap::new_from_string(&context, received_keymap, format, KEYMAP_COMPILE_NO_FLAGS)
            .unwrap();
    println!("{}", keymap.get_as_string(format));

    /*
    let mut file = File::open("./received_keymap.txt").unwrap();
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;
    let keymap =
        Keymap::new_from_string(&context, decoded, format, KEYMAP_COMPILE_NO_FLAGS).unwrap();
    println!("{}", keymap.get_as_string(format));
    */
}
