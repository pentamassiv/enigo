use std::fs::File;

use xkbcommon::xkb::{CONTEXT_NO_FLAGS, Context, FORMAT_TEXT_V1, KEYMAP_COMPILE_NO_FLAGS, Keymap};

fn main() {
    let mut file = File::open("./received_keymap.txt").unwrap();
    let context = Context::new(CONTEXT_NO_FLAGS);
    let format = FORMAT_TEXT_V1;
    let keymap =
        Keymap::new_from_file(&context, &mut file, format, KEYMAP_COMPILE_NO_FLAGS).unwrap();
    println!("{}", keymap.get_as_string(format))
}
