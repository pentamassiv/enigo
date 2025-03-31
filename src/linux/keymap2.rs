use log::debug;
use xkbcommon::xkb::{Keymap, State};

use crate::{Direction, InputResult, Key, keycodes::ModifierBitflag};

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
    pub fn update_state(&mut self, keycode: u32, direction: Direction) {
        debug!("updating xkb:Keymap");
        todo!()
    }
    pub fn update_modifiers(&mut self, new_modifier_bitflag: u32) {
        debug!("updating xkb:Keymap");
        todo!()
    }
    pub fn get_file(&self) -> (u32, ::std::os::unix::io::BorrowedFd<'_>, u32) {
        debug!("updating xkb:Keymap");
        todo!()
    }
    pub fn is_modifier(&self, keycode: u16) -> Option<ModifierBitflag> {
        debug!("updating xkb:Keymap");
        todo!()
    }
    pub fn key_to_keycode(&self, key: Key) -> Option<u16> {
        debug!("updating xkb:Keymap");
        todo!()
    }
    pub fn map_key(&self, key: Key) -> InputResult<u16> {
        debug!("updating xkb:Keymap");
        todo!()
    }
}

impl Default for Keymap2 {
    fn default() -> Self {
        todo!()
    }
}
