//! Fullscreen approval details must not decide the protected request.

use super::*;

pub(super) fn exercise_read_only_details(
    writer: &mut impl Write,
    output_rx: &mpsc::Receiver<Vec<u8>>,
    output: &mut String,
    ledger_path: &Path,
) {
    writer
        .write_all(b"\x01")
        .expect("open approval details with Ctrl-A");
    writer.flush().unwrap();
    wait_for_screen_marker(
        output_rx,
        output,
        "/ Approve command?",
        Duration::from_secs(10),
    );
    wait_for_screen_marker(
        output_rx,
        output,
        "printf tui-gate-b",
        Duration::from_secs(10),
    );
    writer
        .write_all(b"\x1b")
        .expect("close details without deciding approval");
    writer.flush().unwrap();
    wait_for_screen(
        output_rx,
        output,
        "approval choices restored after closing read-only details",
        |screen| {
            screen.contains("Enter confirm · Esc cancel") && !screen.contains("/ Approve command?")
        },
    );
    assert!(!terminal_screen_text(output).contains("/ Approve command?"));
    let ledger = std::fs::read_to_string(ledger_path).expect("approval ledger");
    assert!(
        !ledger
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .any(|entry| entry["scenario"] == "approval" && entry["kind"] == "actionRespond"),
        "closing approval details must not resolve the canonical request"
    );
}
