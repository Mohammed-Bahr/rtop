//! Keyboard event interpretation.
//!
//! Maps crossterm key events to [`Action`]s based on the current input
//! mode. Rendering and state mutation live elsewhere.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, Mode};

/// Translate a key press into an action for the given mode.
pub fn handle_key(key: KeyEvent, mode: Mode) -> Action {
    match mode {
        Mode::Dialog => dialog_key(key),
        Mode::Searching => searching_key(key),
        Mode::Normal => normal_key(key),
    }
}

fn dialog_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => Action::ConfirmYes,
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Backspace => {
            Action::ConfirmNo
        }
        KeyCode::Enter => Action::ConfirmYes,
        _ => Action::RefreshNow, // no-op: keep dialog open
    }
}

fn searching_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::SearchClear,
        KeyCode::Enter => Action::StartSearch, // commit: exit input mode below
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'u' => Action::SearchClear,
                    'w' => Action::SearchClear,
                    _ => Action::RefreshNow,
                }
            } else if key.modifiers.intersects(KeyModifiers::ALT | KeyModifiers::SUPER) {
                Action::RefreshNow
            } else {
                Action::SearchChar(c)
            }
        }
        // Navigation stays available while typing.
        KeyCode::Down => Action::NextProc,
        KeyCode::Up => Action::PreviousProc,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::PageUp => Action::PageUp,
        _ => Action::RefreshNow,
    }
}

fn normal_key(key: KeyEvent) -> Action {
    // Space toggles freeze globally; guard against modifiers so it does not
    // hijack terminal scroll chords.
    if key.code == KeyCode::Char(' ') && key.modifiers.is_empty() {
        return Action::ToggleFreeze;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
        KeyCode::Esc => Action::CloseOverlay,

        KeyCode::Down | KeyCode::Char('j') => Action::NextProc,
        KeyCode::Up | KeyCode::Char('k') => Action::PreviousProc,
        KeyCode::PageDown | KeyCode::Char('J') => Action::PageDown,
        KeyCode::PageUp | KeyCode::Char('K') => Action::PageUp,
        KeyCode::Home | KeyCode::Char('g') => Action::Home,
        KeyCode::End | KeyCode::Char('G') => Action::End,

        KeyCode::Tab => Action::NextScreen,
        KeyCode::BackTab => Action::PreviousScreen,
        KeyCode::Left | KeyCode::Char('h') => Action::PreviousScreen,
        KeyCode::Right | KeyCode::Char('l') => Action::NextScreen,

        KeyCode::Enter => Action::OpenDetails,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('/') => Action::StartSearch,

        KeyCode::Char('s') => Action::CycleSort,
        KeyCode::Char('S') => Action::ToggleSortDirection,
        KeyCode::Char('r') | KeyCode::Char('R') => Action::RefreshNow,

        KeyCode::Char('t') => Action::SendSignal(crate::action::Signal::Term),
        KeyCode::Char('x') | KeyCode::Char('X') => Action::SendSignal(crate::action::Signal::Kill),
        KeyCode::Char('p') => Action::SendSignal(crate::action::Signal::Stop),
        KeyCode::Char('c') => Action::SendSignal(crate::action::Signal::Cont),

        KeyCode::Char('e') => Action::ToggleExpand, // tree expand/collapse

        _ => Action::RefreshNow, // unhandled keys are ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Signal;
    use crossterm::event::KeyEventState;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn normal_mode_keys() {
        assert_eq!(handle_key(key(KeyCode::Char('q')), Mode::Normal), Action::Quit);
        assert_eq!(
            handle_key(key(KeyCode::Char('j')), Mode::Normal),
            Action::NextProc
        );
        assert_eq!(handle_key(key(KeyCode::Down), Mode::Normal), Action::NextProc);
        assert_eq!(handle_key(key(KeyCode::Tab), Mode::Normal), Action::NextScreen);
        assert_eq!(handle_key(key(KeyCode::Char('/')), Mode::Normal), Action::StartSearch);
        assert_eq!(handle_key(key(KeyCode::Char('s')), Mode::Normal), Action::CycleSort);
        assert_eq!(
            handle_key(key(KeyCode::Char('x')), Mode::Normal),
            Action::SendSignal(Signal::Kill)
        );
    }

    #[test]
    fn space_toggles_freeze_in_normal_mode() {
        assert_eq!(
            handle_key(key(KeyCode::Char(' ')), Mode::Normal),
            Action::ToggleFreeze
        );
    }

    #[test]
    fn space_types_a_space_in_search_mode() {
        assert_eq!(
            handle_key(key(KeyCode::Char(' ')), Mode::Searching),
            Action::SearchChar(' ')
        );
    }

    #[test]
    fn search_mode_input() {
        assert_eq!(
            handle_key(key(KeyCode::Char('a')), Mode::Searching),
            Action::SearchChar('a')
        );
        assert_eq!(
            handle_key(key(KeyCode::Backspace), Mode::Searching),
            Action::SearchBackspace
        );
        assert_eq!(handle_key(key(KeyCode::Esc), Mode::Searching), Action::SearchClear);
        // Freeze shortcut must not fire while typing a query containing spaces.
        assert_ne!(
            handle_key(key(KeyCode::Char(' ')), Mode::Searching),
            Action::ToggleFreeze
        );
    }

    #[test]
    fn dialog_mode_answers() {
        assert_eq!(handle_key(key(KeyCode::Char('y')), Mode::Dialog), Action::ConfirmYes);
        assert_eq!(handle_key(key(KeyCode::Char('n')), Mode::Dialog), Action::ConfirmNo);
        assert_eq!(handle_key(key(KeyCode::Esc), Mode::Dialog), Action::ConfirmNo);
        assert_eq!(handle_key(key(KeyCode::Enter), Mode::Dialog), Action::ConfirmYes);
    }
}
