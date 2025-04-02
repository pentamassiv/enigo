use std::{
    collections::HashSet,
    fs::File,
    io::{Read as _, Write as _},
    os::fd::OwnedFd,
};

use log::{debug, error, trace};
use xkbcommon::xkb::{
    Context, KEYMAP_COMPILE_NO_FLAGS, KEYMAP_FORMAT_TEXT_V1, KeyDirection, Keycode, Keymap,
    KeymapFormat, STATE_LAYOUT_DEPRESSED, STATE_LAYOUT_LATCHED, STATE_LAYOUT_LOCKED,
    STATE_MODS_DEPRESSED, STATE_MODS_LATCHED, STATE_MODS_LOCKED, State,
};
use xkeysym::Keysym;

use crate::{InputResult, Key, keycodes::ModifierBitflag};

mod parse_keymap;
pub use parse_keymap::ParsedKeymap;

pub struct Keymap2 {
    context: Context,
    keymap: Keymap,
    state: State,
    parsed_keymap: ParsedKeymap,
    pressed_keys: HashSet<Keycode>,
    keymap_file: Option<File>,
}

impl Keymap2 {
    pub fn new(context: Context, format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<Self, ()> {
        debug!("creating new xkb:Keymap");
        debug!("new(format: {format}, size: {size}, ...)");

        let mut keymap_file = File::from(fd);

        // Check if the file size is correct
        let metadata = keymap_file.metadata().map_err(|e| {
            error!("could not get the file's metadata! Skipping file size check. Error: {e}");
        })?;
        if metadata.len() != size.into() {
            error!("file does not have the expected size! resetting the keymap");
            return Err(());
        }

        let parsed_keymap = ParsedKeymap::try_from(&mut keymap_file).map_err(|_| {
            trace!("unable to parse the new keymap");
        })?;
        // Unfortunately we need to serialize the parsed keymap again, because the
        // xkbcommon parser is super strict and can't handle missing newlines. Ours
        // doesn't mind and when we serialize it, the newlines are added at the correct
        // places so xkbcommon can parse it too
        let keymap_string = format!("{parsed_keymap}");
        let keymap =
            Keymap::new_from_string(&context, keymap_string, format, KEYMAP_COMPILE_NO_FLAGS)
                .ok_or({
                    error!("file does not have the expected size! resetting the keymap");
                })?;

        let state = State::new(&keymap);

        Ok(Self {
            context,
            keymap,
            state,
            parsed_keymap,
            pressed_keys: HashSet::with_capacity(8),
            keymap_file: Some(keymap_file),
        })
    }

    pub fn update(&mut self, format: KeymapFormat, fd: OwnedFd, size: u32) -> Result<(), ()> {
        let depressed_mods = self.state.serialize_mods(STATE_MODS_DEPRESSED);
        let latched_mods = self.state.serialize_mods(STATE_MODS_LATCHED);
        let locked_mods = self.state.serialize_mods(STATE_MODS_LOCKED);

        let depressed_layout = self.state.serialize_layout(STATE_LAYOUT_DEPRESSED);
        let latched_layout = self.state.serialize_layout(STATE_LAYOUT_LATCHED);
        let locked_layout = self.state.serialize_layout(STATE_LAYOUT_LOCKED);

        let Keymap2 {
            context,
            keymap,
            mut state,
            parsed_keymap,
            pressed_keys,
            keymap_file,
        } = Self::new(self.context.clone(), format, fd, size).map_err(|_| {
            trace!("unable to create new keymap");
        })?;

        // The docs say this is a bad idea. update_key and update_mask should not get
        // mixed. I don't know how else to get the same state though
        for key in pressed_keys {
            state.update_key(key, KeyDirection::Down);
        }

        state.update_mask(
            depressed_mods,
            latched_mods,
            locked_mods,
            depressed_layout,
            latched_layout,
            locked_layout,
        );

        self.context = context;
        self.keymap = keymap;
        self.state = state;
        self.parsed_keymap = parsed_keymap;
        self.keymap_file = keymap_file;

        Ok(())
    }

    pub fn update_key(&mut self, keycode: Keycode, direction: KeyDirection) {
        match direction {
            KeyDirection::Up => {
                self.pressed_keys.remove(&keycode);
            }
            KeyDirection::Down => {
                self.pressed_keys.insert(keycode);
            }
        };
        self.state.update_key(keycode, direction);
    }

    pub fn update_modifiers(&mut self, new_modifier_bitflag: u32) {
        debug!("updating xkb:Keymap");
        todo!()
    }

    pub fn format_file_size(&self) -> Result<(KeymapFormat, File, u32), ()> {
        let mut keymap_file = tempfile::tempfile().map_err(|e| {
            error!("could not create temporary file. Error: {e}");
        })?;
        write!(keymap_file, "{}", self.parsed_keymap).map_err(|e| {
            error!("could not write to temporary file. Error: {e}");
        })?;
        let metadata = keymap_file.metadata().map_err(|e| {
            error!("could not get the file's metadata! Error: {e}");
        })?;
        let size = metadata.len().try_into().map_err(|_| {
            error!(
                "keymap file is {} but the maximum is {} (u32::MAX)",
                metadata.len(),
                u32::MAX
            );
        })?;

        let format = KEYMAP_FORMAT_TEXT_V1;

        Ok((format, keymap_file, size))
    }

    pub fn is_modifier(&self, keycode: u16) -> Option<ModifierBitflag> {
        debug!("updating xkb:Keymap");
        todo!()
    }

    pub fn key_to_keycode(&self, key: Key) -> Option<u16> {
        Keysym::from(key)
            .name()
            .and_then(|name| self.keymap.key_by_name(name).map(|k| k.raw()))
            .and_then(|raw| u16::try_from(raw).ok())
    }

    pub fn map_key(&mut self, key: Key) -> InputResult<u16> {
        let key_name = Keysym::from(key).name().ok_or_else(|| {
            crate::InputError::Mapping("the key to map doesn't have a name".to_string())
        })?;
        self.parsed_keymap.map_key(key_name)
    }

    /*
    fn new_xkb_keymap(
        context: &Context,
        format: KeymapFormat,
        keymap_file: &mut File,
        size: u32,
    ) -> Result<Keymap, ()> {
        // Check if the file size is correct.
        let metadata = keymap_file.metadata().map_err(|e| {
            error!("could not get the file's metadata! Skipping file size check. Error: {e}");
        })?;
        if metadata.len() != size.into() {
            error!("file does not have the expected size! resetting the keymap");
            return Err(());
        }

        let flags = KEYMAP_COMPILE_NO_FLAGS;

        // There is a difference between reading the string from the file and creating
        // the keymap from the string and directly creating the keymap from a file. The
        // difference is in the first bytes of the file
        let mut file_content = String::new();
        keymap_file
            .read_to_string(&mut file_content)
            .ok()
            .and_then(|_| {
                Keymap::new_from_string(&context, file_content, format, KEYMAP_COMPILE_NO_FLAGS)
            })
            .or_else(|| Keymap::new_from_file(context, keymap_file, format, flags))
            .ok_or_else(|| error!("Creating xkb::Keymap failed! resetting the keymap"))
    }*/
}

impl Default for Keymap2 {
    fn default() -> Self {
        todo!()
    }
}
