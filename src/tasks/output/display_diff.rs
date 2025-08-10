extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use crate::tasks::output::display::DisplayCommand;

/// Tracks the current state of what's displayed on the LCD
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayState {
    pub line1: String,
    pub line2: String,
    pub cursor_row: u8,
    pub cursor_col: u8,
}

impl Default for DisplayState {
    fn default() -> Self {
        Self {
            line1: String::new(),
            line2: String::new(),
            cursor_row: 0,
            cursor_col: 0,
        }
    }
}

impl DisplayState {
    /// Clear the display state
    pub fn clear(&mut self) {
        self.line1.clear();
        self.line2.clear();
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Update state based on a display command
    pub fn apply_command(&mut self, command: &DisplayCommand) {
        match command {
            DisplayCommand::WriteText { line1, line2 } => {
                self.line1 = line1.clone();
                self.line2 = line2.clone();
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            DisplayCommand::Clear => {
                self.clear();
            }
            DisplayCommand::SetCursor { row, col } => {
                self.cursor_row = *row;
                self.cursor_col = *col;
            }
            DisplayCommand::WriteAt { text, row, col } => {
                // Update the appropriate line at the specified position
                let target_line = if *row == 0 { &mut self.line1 } else { &mut self.line2 };
                
                // Ensure the line is long enough
                while target_line.len() < (*col as usize) {
                    target_line.push(' ');
                }
                
                // Replace characters at the specified position
                let mut chars: Vec<char> = target_line.chars().collect();
                let text_chars: Vec<char> = text.chars().collect();
                
                for (i, &ch) in text_chars.iter().enumerate() {
                    let pos = (*col as usize) + i;
                    if pos < chars.len() {
                        chars[pos] = ch;
                    } else {
                        chars.push(ch);
                    }
                }
                
                *target_line = chars.into_iter().collect();
                self.cursor_row = *row;
                self.cursor_col = *col + text.len() as u8;
            }
            DisplayCommand::ScrollText { .. } => {
                // For scrolling, we can't easily track the exact state,
                // so we'll clear and let the command handle it
                self.clear();
            }
        }
    }
}

/// Display diffing engine that only sends commands when content changes
pub struct DisplayDiffer {
    current_state: DisplayState,
}

impl Default for DisplayDiffer {
    fn default() -> Self {
        Self {
            current_state: DisplayState::default(),
        }
    }
}

impl DisplayDiffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compare desired command with current state and return command if different
    pub fn diff_command(&mut self, command: &DisplayCommand) -> Option<DisplayCommand> {
        match command {
            DisplayCommand::WriteText { line1, line2 } => {
                // Check if the text content is different
                if self.current_state.line1 != *line1 || self.current_state.line2 != *line2 {
                    self.current_state.apply_command(command);
                    Some(command.clone())
                } else {
                    None
                }
            }
            DisplayCommand::Clear => {
                // Only clear if there's actually content to clear
                if !self.current_state.line1.is_empty() || !self.current_state.line2.is_empty() {
                    self.current_state.clear();
                    Some(command.clone())
                } else {
                    None
                }
            }
            DisplayCommand::SetCursor { row, col } => {
                // Only move cursor if position is different
                if self.current_state.cursor_row != *row || self.current_state.cursor_col != *col {
                    self.current_state.apply_command(command);
                    Some(command.clone())
                } else {
                    None
                }
            }
            DisplayCommand::WriteAt { text: _, row: _, col: _ } => {
                // Create a temporary state to see what would change
                let mut temp_state = self.current_state.clone();
                temp_state.apply_command(command);
                
                // Check if the resulting state is different
                if temp_state != self.current_state {
                    self.current_state = temp_state;
                    Some(command.clone())
                } else {
                    None
                }
            }
            DisplayCommand::ScrollText { .. } => {
                // Always allow scroll commands through, as they're dynamic
                self.current_state.apply_command(command);
                Some(command.clone())
            }
        }
    }

    /// Get the current display state (for debugging/testing)
    pub fn current_state(&self) -> &DisplayState {
        &self.current_state
    }

    /// Force reset the state (useful if display is externally modified)
    pub fn reset_state(&mut self) {
        self.current_state = DisplayState::default();
    }
}