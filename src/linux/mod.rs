use xkbcommon::xkb::Keysym;
/// The "empty" keyboard symbol.
// TODO: Replace it with the NO_SYMBOL from xkbcommon, once it is available
// there
pub const NO_SYMBOL: Keysym = Keysym::new(0);

use crate::{Key, KeyboardControllable, MouseButton, MouseControllable};

#[cfg_attr(feature = "x11rb", path = "x11rb.rs")]
#[cfg_attr(not(feature = "x11rb"), path = "xdo.rs")]
mod x11;

#[cfg(feature = "wayland")]
pub mod wayland;

#[derive(Debug)]
pub enum ConnectionError {
    MappingFailed(Keysym),
    Connection(String),
    Format(std::io::Error),
    General(String),
    LostConnection,
    NoKeycode,
    SetLayoutFailed(String),
    Unimplemented,
    Utf(std::string::FromUtf8Error),
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::MappingFailed(e) => write!(f, "Allocation failed: {e:?}"),
            ConnectionError::Connection(e) => write!(f, "Connection: {e}"),
            ConnectionError::Format(e) => write!(f, "Format: {e}"),
            ConnectionError::General(e) => write!(f, "General: {e}"),
            ConnectionError::LostConnection => write!(f, "Lost connection"),
            ConnectionError::NoKeycode => write!(f, "No keycode mapped"),
            ConnectionError::SetLayoutFailed(e) => write!(f, "set_layout() failed: {e}"),
            ConnectionError::Unimplemented => write!(f, "Unimplemented"),
            ConnectionError::Utf(e) => write!(f, "UTF: {e}"),
        }
    }
}

impl From<std::io::Error> for ConnectionError {
    fn from(e: std::io::Error) -> Self {
        ConnectionError::Format(e)
    }
}

pub struct Enigo {
    #[cfg(feature = "wayland")]
    wayland: Option<wayland::Con>,
    x11: Option<x11::Con>,
}

impl Enigo {
    /// Get the delay per keypress.
    /// Default value is 12.
    /// This is Linux-specific.
    #[must_use]
    pub fn delay(&self) -> u32 {
        self.x11.as_ref().unwrap().delay()
    }
    /// Set the delay per keypress.
    /// This is Linux-specific.
    pub fn set_delay(&mut self, delay: u32) {
        self.x11.as_mut().unwrap().set_delay(delay);
    }
}

impl Default for Enigo {
    /// Create a new `Enigo` instance
    fn default() -> Self {
        #[cfg(feature = "wayland")]
        let wayland = wayland::Con::new().ok();
        let x11 = Some(x11::Con::default());
        Self {
            #[cfg(feature = "wayland")]
            wayland,
            x11,
        }
    }
}

impl MouseControllable for Enigo {
    fn mouse_move_to(&mut self, x: i32, y: i32) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_move_to(x, y);
        }
        self.x11.as_mut().unwrap().mouse_move_to(x, y);
    }
    fn mouse_move_relative(&mut self, x: i32, y: i32) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_move_relative(x, y);
        }
        self.x11.as_mut().unwrap().mouse_move_relative(x, y);
    }
    fn mouse_down(&mut self, button: MouseButton) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_down(button);
        }
        self.x11.as_mut().unwrap().mouse_down(button);
    }
    fn mouse_up(&mut self, button: MouseButton) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_up(button);
        }
        self.x11.as_mut().unwrap().mouse_up(button);
    }
    fn mouse_click(&mut self, button: MouseButton) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_click(button);
        }
        self.x11.as_mut().unwrap().mouse_click(button);
    }
    fn mouse_scroll_x(&mut self, length: i32) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_scroll_x(length);
        }
        self.x11.as_mut().unwrap().mouse_scroll_x(length);
    }
    fn mouse_scroll_y(&mut self, length: i32) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.mouse_scroll_y(length);
        }
        self.x11.as_mut().unwrap().mouse_scroll_y(length);
    }
    fn main_display_size(&self) -> (i32, i32) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_ref() {
            return wayland.main_display_size();
        }
        self.x11.as_ref().unwrap().main_display_size()
    }
    fn mouse_location(&self) -> (i32, i32) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_ref() {
            return wayland.mouse_location();
        }
        self.x11.as_ref().unwrap().mouse_location()
    }
}

impl KeyboardControllable for Enigo {
    fn key_sequence(&mut self, sequence: &str) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.key_sequence(sequence);
        }
        self.x11.as_mut().unwrap().key_sequence(sequence);
    }
    fn key_down(&mut self, key: Key) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.key_down(key);
        }
        self.x11.as_mut().unwrap().key_down(key);
    }
    fn key_up(&mut self, key: Key) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.key_up(key);
        }
        self.x11.as_mut().unwrap().key_up(key);
    }
    fn key_click(&mut self, key: Key) {
        #[cfg(feature = "wayland")]
        if let Some(wayland) = self.wayland.as_mut() {
            wayland.key_click(key);
        }
        self.x11.as_mut().unwrap().key_click(key);
    }
}
