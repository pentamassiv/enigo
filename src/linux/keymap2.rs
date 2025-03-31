use log::debug;
use xkbcommon::xkb::{Keymap, State};

#[derive(Clone)]
pub struct Keymap2 {
    keymap: Keymap,
    state: State,
}

impl Keymap2 {
    pub fn new() -> Self {
        debug!("creating new xkb:Keymap");
        todo!();
    }
    pub fn update(&mut self, new_keymap: Keymap) {
        debug!("updating xkb:Keymap");
        todo!()
    }
}

impl Default for Keymap2 {
    fn default() -> Self {
        todo!()
    }
}
