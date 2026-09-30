use base64::Engine;
use std::fmt;
use std::io::Write;

pub(crate) mod worker;

const OSC52_MAX_RAW_BYTES: usize = 100_000;

#[cfg(target_os = "macos")]
static STDERR_SUPPRESSION_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> =
    std::sync::OnceLock::new();

pub(crate) struct ClipboardLease {
    #[cfg(target_os = "linux")]
    _clipboard: Option<arboard::Clipboard>,
}

/// A confirmed native clipboard write or an unacknowledged terminal request.
pub(crate) enum CopyOutcome {
    Copied(Option<ClipboardLease>),
    Requested,
}

impl CopyOutcome {
    /// Preserve an existing Linux clipboard owner unless a confirmed write replaces it.
    pub(crate) fn store(self, lease: &mut Option<ClipboardLease>) -> CopyStatus {
        match self {
            Self::Copied(next_lease) => {
                if let Some(next_lease) = next_lease {
                    *lease = Some(next_lease);
                }
                CopyStatus::Confirmed
            }
            Self::Requested => CopyStatus::Unconfirmed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyStatus {
    Confirmed,
    Unconfirmed,
}

impl fmt::Debug for ClipboardLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ClipboardLease")
    }
}

impl ClipboardLease {
    #[cfg(target_os = "linux")]
    fn native_linux(clipboard: arboard::Clipboard) -> Self {
        Self {
            _clipboard: Some(clipboard),
        }
    }

    #[cfg(test)]
    pub(crate) fn test() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            _clipboard: None,
        }
    }
}

#[derive(Clone, Copy)]
struct CopyEnvironment {
    ssh_session: bool,
    tmux_session: bool,
    wsl_session: bool,
}

pub(crate) fn copy_to_clipboard(text: &str) -> Result<CopyOutcome, String> {
    copy_to_clipboard_with(
        text,
        CopyEnvironment {
            ssh_session: is_ssh_session(),
            tmux_session: is_tmux_session(),
            wsl_session: is_wsl_session(),
        },
        tmux_clipboard_copy,
        osc52_copy,
        arboard_copy,
        wsl_clipboard_copy,
    )
}

/// Publish text to the X11 PRIMARY selection.
///
/// PRIMARY is intentionally separate from CLIPBOARD: X11 requires the process to keep the
/// native owner alive until another application requests the selection.  Non-X11 terminals fail
/// closed so a remote or Wayland session never reports a misleading successful middle-click
/// paste.  The caller owns the returned lease independently from the regular clipboard lease.
pub(crate) fn copy_to_primary(text: &str) -> Result<CopyOutcome, String> {
    if text.is_empty() {
        return Err("nothing to copy: the selected content is empty".to_string());
    }
    if !crate::clipboard_paste::mouse_paste_source_allowed(
        crate::clipboard_paste::ClipboardTextSource::Primary,
    ) {
        return Err("X11 primary selection is unavailable in this terminal".to_string());
    }
    #[cfg(target_os = "linux")]
    {
        use arboard::SetExtLinux;

        let _guard = SuppressStderr::new();
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|error| format!("primary selection unavailable: {error}"))?;
        clipboard
            .set()
            .clipboard(arboard::LinuxClipboardKind::Primary)
            .text(text)
            .map_err(|error| format!("failed to set X11 primary selection: {error}"))?;
        return Ok(CopyOutcome::Copied(Some(ClipboardLease::native_linux(
            clipboard,
        ))));
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = text;
        Err("X11 primary selection is unavailable on this platform".to_string())
    }
}

fn copy_to_clipboard_with(
    text: &str,
    environment: CopyEnvironment,
    tmux_copy: impl Fn(&str) -> Result<(), String>,
    osc52_copy: impl Fn(&str) -> Result<(), String>,
    native_copy: impl Fn(&str) -> Result<Option<ClipboardLease>, String>,
    wsl_copy: impl Fn(&str) -> Result<(), String>,
) -> Result<CopyOutcome, String> {
    if text.is_empty() {
        return Err("nothing to copy: the selected content is empty".to_string());
    }

    let terminal_copy = || terminal_copy(text, environment.tmux_session, &tmux_copy, &osc52_copy);
    let native_result = native_copy(text).or_else(|native_error| {
        if environment.wsl_session {
            wsl_copy(text).map(|()| None).map_err(|wsl_error| {
                format!("native clipboard: {native_error}; WSL fallback: {wsl_error}")
            })
        } else {
            Err(format!("native clipboard: {native_error}"))
        }
    });
    // Terminal writes have no delivery acknowledgement, so they never suppress a native attempt.
    // tmux forwarding also runs for local sessions because future attached clients need the copy.
    let terminal_result = (environment.tmux_session || environment.ssh_session).then(terminal_copy);
    match native_result {
        Ok(lease) => Ok(CopyOutcome::Copied(lease)),
        Err(native_error) => terminal_result
            .unwrap_or_else(terminal_copy)
            .map(|()| CopyOutcome::Requested)
            .map_err(|terminal_error| {
                format!("{native_error}; terminal clipboard: {terminal_error}")
            }),
    }
}

fn terminal_copy(
    text: &str,
    tmux_session: bool,
    tmux_copy: &impl Fn(&str) -> Result<(), String>,
    osc52_copy: &impl Fn(&str) -> Result<(), String>,
) -> Result<(), String> {
    if tmux_session {
        return tmux_copy(text).or_else(|tmux_error| {
            osc52_copy(text).map_err(|osc52_error| {
                format!("tmux clipboard: {tmux_error}; OSC 52 fallback: {osc52_error}")
            })
        });
    }
    osc52_copy(text)
}

pub(crate) fn is_ssh_session() -> bool {
    std::env::var_os("SSH_TTY").is_some() || std::env::var_os("SSH_CONNECTION").is_some()
}

pub(crate) fn is_tmux_session() -> bool {
    std::env::var_os("TMUX").is_some() || std::env::var_os("TMUX_PANE").is_some()
}

#[cfg(target_os = "linux")]
pub(crate) fn is_wsl_session() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|release| release.to_ascii_lowercase().contains("microsoft"))
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn is_wsl_session() -> bool {
    false
}

#[cfg(all(not(target_os = "android"), not(target_os = "linux")))]
fn arboard_copy(text: &str) -> Result<Option<ClipboardLease>, String> {
    #[cfg(target_os = "macos")]
    let _stderr_lock = STDERR_SUPPRESSION_MUTEX
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .map_err(|_| "stderr suppression lock poisoned".to_string())?;
    let _guard = SuppressStderr::new();
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("clipboard unavailable: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("failed to set clipboard text: {error}"))?;
    Ok(None)
}

#[cfg(target_os = "linux")]
fn arboard_copy(text: &str) -> Result<Option<ClipboardLease>, String> {
    let _guard = SuppressStderr::new();
    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("clipboard unavailable: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("failed to set clipboard text: {error}"))?;
    Ok(Some(ClipboardLease::native_linux(clipboard)))
}

#[cfg(target_os = "android")]
fn arboard_copy(_text: &str) -> Result<Option<ClipboardLease>, String> {
    Err("native clipboard unavailable on Android".to_string())
}

#[cfg(target_os = "linux")]
fn wsl_clipboard_copy(text: &str) -> Result<(), String> {
    let mut child = std::process::Command::new("powershell.exe")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .args([
            "-NoProfile",
            "-Command",
            "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; $ErrorActionPreference = 'Stop'; $text = [Console]::In.ReadToEnd(); Set-Clipboard -Value $text",
        ])
        .spawn()
        .map_err(|error| format!("failed to spawn powershell.exe: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open powershell.exe stdin".to_string())?;
    stdin
        .write_all(text.as_bytes())
        .map_err(|error| format!("failed to write to powershell.exe: {error}"))?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for powershell.exe: {error}"))?;
    output.status.success().then_some(()).ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            format!("powershell.exe exited with status {}", output.status)
        } else {
            format!("powershell.exe failed: {stderr}")
        }
    })
}

#[cfg(not(target_os = "linux"))]
fn wsl_clipboard_copy(_text: &str) -> Result<(), String> {
    Err("WSL clipboard fallback unavailable on this platform".to_string())
}

fn tmux_clipboard_copy(text: &str) -> Result<(), String> {
    tmux_clipboard_copy_ready(
        || tmux_command_output(["show-options", "-gv", "set-clipboard"]),
        || tmux_command_output(["info"]),
    )?;
    let mut child = std::process::Command::new("tmux")
        .args(["load-buffer", "-w", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn tmux: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "failed to open tmux stdin".to_string())?;
    stdin
        .write_all(text.as_bytes())
        .map_err(|error| format!("failed to write to tmux: {error}"))?;
    drop(stdin);
    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed to wait for tmux: {error}"))?;
    output.status.success().then_some(()).ok_or_else(|| {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            format!("tmux exited with status {}", output.status)
        } else {
            format!("tmux failed: {stderr}")
        }
    })
}

fn tmux_clipboard_copy_ready(
    set_clipboard: impl FnOnce() -> Result<String, String>,
    terminal_info: impl FnOnce() -> Result<String, String>,
) -> Result<(), String> {
    if set_clipboard()?.trim() == "off" {
        return Err("tmux clipboard forwarding is disabled".to_string());
    }
    if terminal_info()?
        .lines()
        .any(|line| line.contains("Ms: [missing]"))
    {
        return Err("tmux clipboard forwarding is unavailable: missing Ms capability".to_string());
    }
    Ok(())
}

fn tmux_command_output<const N: usize>(args: [&str; N]) -> Result<String, String> {
    let output = std::process::Command::new("tmux")
        .args(args)
        .output()
        .map_err(|error| format!("failed to spawn tmux: {error}"))?;
    if output.status.success() {
        String::from_utf8(output.stdout)
            .map_err(|error| format!("tmux output was not UTF-8: {error}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(format!("tmux exited with status {}", output.status))
        } else {
            Err(format!("tmux failed: {stderr}"))
        }
    }
}

fn osc52_copy(text: &str) -> Result<(), String> {
    let sequence = osc52_sequence(text, is_tmux_session())?;
    #[cfg(unix)]
    if let Ok(tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        if write_osc52(tty, &sequence).is_ok() {
            return Ok(());
        }
    }
    write_osc52(std::io::stdout().lock(), &sequence)
}

fn write_osc52(mut writer: impl Write, sequence: &str) -> Result<(), String> {
    writer
        .write_all(sequence.as_bytes())
        .map_err(|error| format!("failed to write OSC 52: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("failed to flush OSC 52: {error}"))
}

fn osc52_sequence(text: &str, tmux: bool) -> Result<String, String> {
    if text.len() > OSC52_MAX_RAW_BYTES {
        return Err(format!(
            "OSC 52 payload too large ({} bytes; max {OSC52_MAX_RAW_BYTES})",
            text.len()
        ));
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    if tmux {
        Ok(format!("\x1bPtmux;\x1b\x1b]52;c;{encoded}\x07\x1b\\"))
    } else {
        Ok(format!("\x1b]52;c;{encoded}\x07"))
    }
}

#[cfg(target_os = "macos")]
struct SuppressStderr {
    saved_fd: Option<libc::c_int>,
}

#[cfg(target_os = "macos")]
impl SuppressStderr {
    fn new() -> Self {
        unsafe {
            let saved = libc::dup(2);
            if saved < 0 {
                return Self { saved_fd: None };
            }
            let devnull = libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY);
            if devnull < 0 || libc::dup2(devnull, 2) < 0 {
                libc::close(saved);
                if devnull >= 0 {
                    libc::close(devnull);
                }
                return Self { saved_fd: None };
            }
            libc::close(devnull);
            Self {
                saved_fd: Some(saved),
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for SuppressStderr {
    fn drop(&mut self) {
        if let Some(saved) = self.saved_fd {
            unsafe {
                libc::dup2(saved, 2);
                libc::close(saved);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
struct SuppressStderr;

#[cfg(not(target_os = "macos"))]
impl SuppressStderr {
    fn new() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn osc52_encoding_roundtrips_and_wraps_tmux() {
        let text = "# Result\n\n```rust\nfn main() {}\n```";
        let sequence = osc52_sequence(text, false).expect("OSC 52 sequence");
        let encoded = sequence
            .trim_start_matches("\u{1b}]52;c;")
            .trim_end_matches('\u{7}');
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .expect("base64"),
            text.as_bytes()
        );
        assert_eq!(
            osc52_sequence("hello", true),
            Ok("\u{1b}Ptmux;\u{1b}\u{1b}]52;c;aGVsbG8=\u{7}\u{1b}\\".to_string())
        );
    }

    #[test]
    fn osc52_rejects_oversized_payloads() {
        let text = "x".repeat(OSC52_MAX_RAW_BYTES + 1);
        assert!(osc52_sequence(&text, false).is_err());
    }

    #[test]
    fn primary_selection_rejects_empty_text_before_platform_probe() {
        match copy_to_primary("") {
            Err(error) => assert_eq!(error, "nothing to copy: the selected content is empty"),
            Ok(_) => panic!("empty selection must fail"),
        }
    }

    #[test]
    fn ssh_attempts_native_and_terminal_while_local_prefers_native() {
        let native_calls = Cell::new(0);
        let osc_calls = Cell::new(0);
        let remote = copy_to_clipboard_with(
            "remote",
            CopyEnvironment {
                ssh_session: true,
                tmux_session: false,
                wsl_session: false,
            },
            |_| Ok(()),
            |_| {
                osc_calls.set(osc_calls.get() + 1);
                Ok(())
            },
            |_| {
                native_calls.set(native_calls.get() + 1);
                Ok(None)
            },
            |_| Ok(()),
        );
        assert!(matches!(remote, Ok(CopyOutcome::Copied(None))));
        assert_eq!(osc_calls.get(), 1);
        assert_eq!(native_calls.get(), 1);

        let local = copy_to_clipboard_with(
            "local",
            CopyEnvironment {
                ssh_session: false,
                tmux_session: false,
                wsl_session: false,
            },
            |_| Ok(()),
            |_| panic!("OSC 52 should not run"),
            |_| Ok(Some(ClipboardLease::test())),
            |_| Ok(()),
        );
        assert!(matches!(local, Ok(CopyOutcome::Copied(Some(_)))));
    }

    #[test]
    fn tmux_and_local_failures_fall_back_to_osc52() {
        let osc_calls = Cell::new(0);
        let result = copy_to_clipboard_with(
            "hello",
            CopyEnvironment {
                ssh_session: false,
                tmux_session: true,
                wsl_session: false,
            },
            |_| Err("tmux unavailable".to_string()),
            |_| {
                osc_calls.set(osc_calls.get() + 1);
                Ok(())
            },
            |_| Err("native unavailable".to_string()),
            |_| Ok(()),
        );
        assert!(matches!(result, Ok(CopyOutcome::Requested)));
        assert_eq!(osc_calls.get(), 1);
    }

    #[test]
    fn terminal_only_copy_is_unconfirmed_and_preserves_native_lease() {
        let outcome = copy_to_clipboard_with(
            "hello",
            CopyEnvironment {
                ssh_session: true,
                tmux_session: false,
                wsl_session: false,
            },
            |_| Ok(()),
            |_| Ok(()),
            |_| Err("native unavailable".to_string()),
            |_| Ok(()),
        )
        .expect("terminal request");
        let mut lease = Some(ClipboardLease::test());

        assert_eq!(outcome.store(&mut lease), CopyStatus::Unconfirmed);
        assert!(lease.is_some());
    }

    #[test]
    fn wsl_copy_is_confirmed_and_native_success_wins_over_terminal_failure() {
        let wsl = copy_to_clipboard_with(
            "wsl",
            CopyEnvironment {
                ssh_session: false,
                tmux_session: false,
                wsl_session: true,
            },
            |_| Ok(()),
            |_| panic!("terminal fallback should not run"),
            |_| Err("native unavailable".to_string()),
            |_| Ok(()),
        )
        .expect("WSL clipboard write");
        let mut lease = None;
        assert_eq!(wsl.store(&mut lease), CopyStatus::Confirmed);

        let native = copy_to_clipboard_with(
            "native",
            CopyEnvironment {
                ssh_session: true,
                tmux_session: false,
                wsl_session: false,
            },
            |_| Ok(()),
            |_| Err("terminal blocked".to_string()),
            |_| Ok(Some(ClipboardLease::test())),
            |_| Ok(()),
        )
        .expect("native clipboard write");
        assert_eq!(native.store(&mut lease), CopyStatus::Confirmed);
        assert!(lease.is_some());
    }

    #[test]
    fn tmux_requires_clipboard_forwarding_capability() {
        assert!(tmux_clipboard_copy_ready(
            || Ok("external\n".to_string()),
            || Ok("193: Ms: (string) \\033]52\n".to_string()),
        )
        .is_ok());
        assert_eq!(
            tmux_clipboard_copy_ready(
                || Ok("off\n".to_string()),
                || panic!("terminal info should not be queried"),
            ),
            Err("tmux clipboard forwarding is disabled".to_string())
        );
        assert_eq!(
            tmux_clipboard_copy_ready(
                || Ok("external\n".to_string()),
                || Ok("193: Ms: [missing]\n".to_string()),
            ),
            Err("tmux clipboard forwarding is unavailable: missing Ms capability".to_string())
        );
    }
}
