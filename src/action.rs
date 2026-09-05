/// Signals the process manager can send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Term,
    Kill,
    Stop,
    Cont,
}

impl Signal {
    /// Raw POSIX signal number on Unix; a stable placeholder on Windows.
    pub fn as_raw(self) -> i32 {
        #[cfg(unix)]
        {
            match self {
                Signal::Term => libc::SIGTERM,
                Signal::Kill => libc::SIGKILL,
                Signal::Stop => libc::SIGSTOP,
                Signal::Cont => libc::SIGCONT,
            }
        }

        #[cfg(not(unix))]
        {
            match self {
                Signal::Term => 15,
                Signal::Kill => 9,
                Signal::Stop => 19,
                Signal::Cont => 18,
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Signal::Term => "SIGTERM",
            Signal::Kill => "SIGKILL",
            Signal::Stop => "SIGSTOP",
            Signal::Cont => "SIGCONT",
        }
    }

    /// Destructive signals require a confirmation dialog before sending.
    pub fn requires_confirmation(self) -> bool {
        matches!(self, Signal::Term | Signal::Kill)
    }
}

/// Everything the user can do, decoupled from key handling so keybindings
/// and UI mode transitions stay in one place (`event.rs`) while `App`
/// owns state mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,

    // Navigation
    NextProc,
    PreviousProc,
    PageUp,
    PageDown,
    Home,
    End,
    NextScreen,
    PreviousScreen,

    // Views / overlays
    OpenDetails,
    CloseOverlay,
    ToggleHelp,

    // Process list
    CycleSort,
    ToggleSortDirection,
    StartSearch,
    SearchChar(char),
    SearchBackspace,
    SearchClear,

    // Tree view
    ToggleExpand,

    // Signals
    SendSignal(Signal),

    // Dialogs
    ConfirmYes,
    ConfirmNo,

    // Misc
    ToggleFreeze,
    RefreshNow,
}

/// Terminal input modes. Only one is active at a time; keys are interpreted
/// differently per mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal navigation.
    Normal,
    /// Incremental search: printable characters append to the query.
    Searching,
    /// A confirmation dialog has focus (y/n or Enter/Esc).
    Dialog,
}
