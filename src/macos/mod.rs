#[macro_use]
extern crate objc;

mod macos_impl;

pub mod keycodes;
pub use self::macos_impl::Enigo;
