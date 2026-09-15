use std::path::PathBuf;
use std::time::Duration;

use anyhow::{ensure, Result};

use super::focus_palette::PtyLime;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const TRANSCRIPT_TIMEOUT: Duration = Duration::from_secs(15);

/// Exercise the real paginated Thread/Turn/Item path through `lime resume` and a PTY.
///
/// The fixture seeds more than one item page, a completed review turn, and an interrupted
/// nested turn with duplicate canonical user messages. Home must load the older page and retain
/// the turn-aware filtering/separator decisions across that boundary.
#[test]
fn older_pagination_reconciles_review_prompts_across_page_boundaries() -> Result<()> {
    if std::env::var_os("LIME_TEST_TUI_HISTORY_PAGINATION").is_none() {
        return Ok(());
    }

    let cli_bin = required_path("LIME_TEST_CLI_BIN");
    let app_server_bin = required_path("LIME_TEST_APP_SERVER_BIN");
    let backend_path = required_path("LIME_TEST_TERMINAL_BACKEND");
    let ledger_path = required_path("LIME_TEST_TERMINAL_LEDGER");
    let cwd = required_path("LIME_TEST_TERMINAL_CWD");
    let node_bin = required_path("LIME_TEST_NODE_BIN");
    let thread_id = required_path("LIME_TEST_TUI_HISTORY_THREAD_ID")
        .to_string_lossy()
        .into_owned();

    let mut terminal = PtyLime::start_resume(
        &cli_bin,
        &app_server_bin,
        &backend_path,
        &ledger_path,
        &cwd,
        &node_bin,
        &thread_id,
    )?;
    terminal.wait_for_startup_with_timeout(STARTUP_TIMEOUT)?;

    // Ctrl-T opens the complete transcript. Home is sent as the terminal's canonical ESC [ H
    // sequence; the pager requests an older page only when the scroll reaches its top.
    terminal.write_input(&[0x14])?;
    terminal.wait_for_screen_compact_contains("TRANSCRIPT", TRANSCRIPT_TIMEOUT)?;
    terminal.write_input(b"\x1b[H")?;
    terminal.wait_for_screen_compact_contains("SEED_000", TRANSCRIPT_TIMEOUT)?;
    let top = terminal.screen_contents();
    ensure!(
        !top.contains("NESTED_REVIEW_PROMPT"),
        "nested review prompt leaked into top transcript page:\n{top}"
    );

    terminal.write_input(b"\x1b[F")?;
    terminal.wait_for_screen_compact_contains("SEED_050", TRANSCRIPT_TIMEOUT)?;
    let bottom = terminal.screen_contents();
    ensure!(
        !bottom.contains("NESTED_REVIEW_PROMPT"),
        "nested review prompt leaked into newest transcript page:\n{bottom}"
    );
    let mut previous_separator = false;
    for line in bottom.lines().filter(|line| !line.trim().is_empty()) {
        let separator = line.trim().starts_with("---");
        ensure!(
            !(separator && previous_separator),
            "completion separators duplicated after cross-page reconciliation:\n{bottom}"
        );
        previous_separator = separator;
    }

    terminal.write_input(&[0x14])?;
    terminal.wait_for_screen_without("TRANSCRIPT", TRANSCRIPT_TIMEOUT)?;
    terminal.write_input(&[0x04])?;
    terminal.wait_for_exit()?;
    ensure!(
        terminal.output_contains(b"\x1b[?1049l"),
        "alternate screen was not restored after history pagination test"
    );
    Ok(())
}
fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("missing {name}"))
}
