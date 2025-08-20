extern crate alloc;

use crate::events::{Command, InputEvent};
use alloc::vec::Vec;
use core::any::Any;

pub mod main_menu;
pub mod search_and_destroy;

pub trait App {
    fn handle_event(&mut self, event: InputEvent);
    fn render(&mut self) -> Vec<Command>;
    fn as_any(&self) -> &dyn Any;
}
