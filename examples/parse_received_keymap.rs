use std::{
    fs::File,
    io::{Read as _, Write},
};

use enigo::ParsedKeymap;
use xkbcommon::xkb::{CONTEXT_NO_FLAGS, Context, FORMAT_TEXT_V1, KEYMAP_COMPILE_NO_FLAGS, Keymap};

fn main() {
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;

    let mut keymap_file = File::open("./binary_keymap_decoded.txt").unwrap();
    let keymap = ParsedKeymap::try_from(&mut keymap_file).unwrap();
    let received_keymap = format!("{keymap}");
    let keymap =
        Keymap::new_from_string(&context, received_keymap, format, KEYMAP_COMPILE_NO_FLAGS);
    if let Some(keymap) = keymap {
        println!("1{}1", keymap.get_as_string(format));
        return;
    }

    let received_keymap = std::fs::read_to_string("binary_keymap_decoded.txt").unwrap();
    println!("{}", received_keymap);
    let keymap =
        Keymap::new_from_string(&context, received_keymap, format, KEYMAP_COMPILE_NO_FLAGS);
    if let Some(keymap) = keymap {
        println!("2{}2", keymap.get_as_string(format));
        return;
    }
}
