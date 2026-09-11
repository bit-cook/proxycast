use std::future::Future;
use std::io::{self, stdout, Stdout, Write};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;

use crossterm::cursor::Show;
#[cfg(not(windows))]
use crossterm::event::EnableFocusChange;
use crossterm::event::{DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, KeyEvent};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Position, Size};
use ratatui::Terminal as RatatuiTerminal;
use tokio::sync::broadcast;

use crate::viewport::ViewportState;

pub(crate) mod event_stream;
mod frame_rate_limiter;
mod frame_requester;

pub(crate) use event_stream::{EventBroker, TuiEventStream};
pub(crate) use frame_requester::FrameRequester;

/// Normalized events consumed by the TUI runtime.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    Key(KeyEvent),
    Paste(String),
    Resize(ratatui::layout::Size),
    Draw,
    #[allow(dead_code)]
    Resume,
    FocusGained,
    FocusLost,
}

pub(crate) type Terminal = RatatuiTerminal<CrosstermBackend<Stdout>>;

static PANIC_HOOK: Once = Once::new();
static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);

fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            if TERMINAL_ACTIVE.swap(false, Ordering::AcqRel) {
                let _ = restore_terminal_state();
            }
            previous(panic_info);
        }));
    });
}

fn restore_terminal_state() -> io::Result<()> {
    let mut output = stdout();
    let mut first_error = crossterm::execute!(output, Show).err();
    if let Err(error) = crossterm::execute!(
        output,
        DisableBracketedPaste,
        DisableFocusChange,
        LeaveAlternateScreen
    ) {
        first_error.get_or_insert(error);
    }
    if let Err(error) = disable_raw_mode() {
        first_error.get_or_insert(error);
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn set_modes() -> io::Result<()> {
    #[cfg(not(windows))]
    execute!(stdout(), EnableBracketedPaste, EnableFocusChange)?;
    #[cfg(windows)]
    execute!(stdout(), EnableBracketedPaste)?;
    enable_raw_mode()
}

fn restore_keep_raw() -> io::Result<()> {
    let mut output = stdout();
    let mut first_error = execute!(output, DisableBracketedPaste, DisableFocusChange).err();
    if let Err(error) = execute!(output, Show) {
        first_error.get_or_insert(error);
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

#[cfg(unix)]
fn flush_terminal_input_buffer() {
    // SAFETY: flushing the stdin input queue does not transfer ownership.
    let result = unsafe { libc::tcflush(libc::STDIN_FILENO, libc::TCIFLUSH) };
    if result != 0 {
        tracing::warn!(
            error = %io::Error::last_os_error(),
            "failed to flush terminal input buffer"
        );
    }
}

#[cfg(windows)]
fn flush_terminal_input_buffer() {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{
        FlushConsoleInputBuffer, GetStdHandle, STD_INPUT_HANDLE,
    };

    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if handle != 0 && handle != INVALID_HANDLE_VALUE && FlushConsoleInputBuffer(handle) == 0 {
            tracing::warn!("failed to flush terminal input buffer");
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn flush_terminal_input_buffer() {}

pub(crate) struct Tui {
    terminal: Terminal,
    viewport: ViewportState,
    restored: bool,
    event_broker: Arc<EventBroker>,
    draw_tx: broadcast::Sender<()>,
    frame_requester: FrameRequester,
    terminal_focused: Arc<AtomicBool>,
}

impl Tui {
    pub(crate) fn enter() -> io::Result<Self> {
        install_panic_hook();
        enable_raw_mode()?;
        let mut output = stdout();
        #[cfg(not(windows))]
        let mode_result = execute!(
            output,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableFocusChange
        );
        #[cfg(windows)]
        let mode_result = execute!(output, EnterAlternateScreen, EnableBracketedPaste);
        if let Err(error) = mode_result {
            let _ = disable_raw_mode();
            return Err(error);
        }
        let terminal = match RatatuiTerminal::new(CrosstermBackend::new(output)) {
            Ok(terminal) => terminal,
            Err(error) => {
                let mut output = stdout();
                #[cfg(not(windows))]
                let _ = execute!(
                    output,
                    DisableBracketedPaste,
                    DisableFocusChange,
                    LeaveAlternateScreen
                );
                #[cfg(windows)]
                let _ = execute!(output, DisableBracketedPaste, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(error);
            }
        };
        let size = terminal.size()?;
        #[cfg(unix)]
        let startup_probe = match crate::terminal_probe::startup(
            crate::terminal_probe::DEFAULT_TIMEOUT,
            crate::terminal_probe::StartupKeyboardEnhancementProbe::Skip,
        ) {
            Ok(probe) => {
                tracing::debug!(
                    cursor_position = probe.cursor_position.is_some(),
                    default_colors = probe.default_colors.is_some(),
                    "terminal startup probes completed"
                );
                probe
            }
            Err(error) => {
                tracing::debug!(%error, "terminal startup probes unavailable");
                crate::terminal_probe::StartupProbe {
                    cursor_position: None,
                    default_colors: None,
                    keyboard_enhancement_supported: None,
                }
            }
        };
        #[cfg(unix)]
        crate::terminal_palette::set_default_colors_from_startup_probe(
            startup_probe.default_colors,
        );
        #[cfg(windows)]
        crate::terminal_palette::set_default_colors_from_startup_probe(
            crate::terminal_probe::default_colors(crate::terminal_probe::DEFAULT_TIMEOUT)
                .ok()
                .flatten(),
        );
        let mut viewport = ViewportState::new(size, {
            #[cfg(unix)]
            {
                startup_probe.cursor_position.unwrap_or(Position::ORIGIN)
            }
            #[cfg(not(unix))]
            {
                Position::ORIGIN
            }
        });
        viewport.enter_alternate_screen(size);
        let event_broker = Arc::new(EventBroker::new());
        let (draw_tx, _) = broadcast::channel(8);
        let frame_requester = FrameRequester::new(draw_tx.clone());
        let terminal_focused = Arc::new(AtomicBool::new(true));
        TERMINAL_ACTIVE.store(true, Ordering::Release);
        Ok(Self {
            terminal,
            viewport,
            restored: false,
            event_broker,
            draw_tx,
            frame_requester,
            terminal_focused,
        })
    }

    pub(crate) fn terminal_mut(&mut self) -> &mut Terminal {
        &mut self.terminal
    }

    pub(crate) fn screen_size(&self) -> Size {
        self.terminal
            .size()
            .unwrap_or_else(|_| self.viewport.area().as_size())
    }

    pub(crate) fn last_known_cursor_position(&mut self) -> Position {
        self.terminal
            .backend_mut()
            .get_cursor_position()
            .unwrap_or(Position::ORIGIN)
    }

    pub(crate) fn write_ansi(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.terminal.backend_mut().write_all(bytes)
    }

    pub(crate) fn set_viewport_area(&mut self, area: ratatui::layout::Rect) {
        self.viewport.set_viewport_area(area);
    }

    pub(crate) fn note_history_rows_inserted(&mut self, rows: u16) {
        self.viewport.note_history_rows_inserted(rows);
    }

    pub(crate) fn update_viewport(&mut self, screen_size: Size, content_height: u16) {
        self.viewport.update_inline(screen_size, content_height);
    }

    pub(crate) fn sync_viewport(&mut self) -> io::Result<()> {
        let screen_size = self.terminal.size()?;
        self.update_viewport(screen_size, screen_size.height);
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn viewport_area(&self) -> ratatui::layout::Rect {
        self.viewport.area()
    }

    pub(crate) fn event_stream(&self) -> TuiEventStream {
        TuiEventStream::new(
            self.event_broker.clone(),
            self.draw_tx.subscribe(),
            self.terminal_focused.clone(),
        )
    }

    pub(crate) fn frame_requester(&self) -> FrameRequester {
        self.frame_requester.clone()
    }

    pub(crate) fn pause_events(&self) {
        self.event_broker.pause_events();
    }

    pub(crate) fn resume_events(&self) {
        self.event_broker.resume_events();
    }

    #[allow(dead_code)]
    pub(crate) fn is_terminal_focused(&self) -> bool {
        self.terminal_focused
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Temporarily restore terminal state while an external interactive program runs.
    pub(crate) async fn with_restored<R, F, Fut>(&mut self, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = R>,
    {
        self.pause_events();

        let was_alt_screen = self.viewport.is_alt_screen_active();
        if was_alt_screen {
            let _ = self.leave_alt_screen();
        }

        if let Err(error) = restore_keep_raw() {
            tracing::warn!(%error, "failed to restore terminal modes before external program");
        }

        let output = f().await;

        if let Err(error) = set_modes() {
            tracing::warn!(%error, "failed to re-enable terminal modes after external program");
        }
        flush_terminal_input_buffer();

        if was_alt_screen {
            let _ = self.enter_alt_screen();
        }

        self.resume_events();
        self.frame_requester.schedule_frame();
        output
    }

    pub(crate) fn restore(&mut self) -> io::Result<()> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        self.event_broker.pause_events();
        TERMINAL_ACTIVE.store(false, Ordering::Release);
        let mut first_error = self.terminal.show_cursor().err();
        if let Err(error) = execute!(
            self.terminal.backend_mut(),
            DisableBracketedPaste,
            DisableFocusChange,
            LeaveAlternateScreen
        ) {
            first_error.get_or_insert(error);
        }
        self.viewport.leave_alternate_screen();
        if let Err(error) = disable_raw_mode() {
            first_error.get_or_insert(error);
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    #[allow(dead_code)]
    /// Enter alternate screen and expand the viewport to full terminal size, saving the current
    /// inline viewport for restoration when leaving.
    pub(crate) fn enter_alt_screen(&mut self) -> io::Result<()> {
        let _ = execute!(self.terminal.backend_mut(), EnterAlternateScreen);
        if let Ok(size) = self.terminal.size() {
            self.viewport.enter_alternate_screen(size);
            self.terminal
                .resize(ratatui::layout::Rect::new(0, 0, size.width, size.height))?;
        }
        Ok(())
    }

    /// Leave alternate screen and restore the previously saved inline viewport, if any.
    pub(crate) fn leave_alt_screen(&mut self) -> io::Result<()> {
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        self.viewport.leave_alternate_screen();
        Ok(())
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
