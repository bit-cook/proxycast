//! Real keyboard completion is observed through PTY screen state, not injected popup models.

use super::*;

pub(super) fn exercise_suggestion_menus(
    writer: &mut impl Write,
    output_rx: &mpsc::Receiver<Vec<u8>>,
    output: &mut String,
    ledger_path: &Path,
) {
    write_typed_text(writer, b"/");
    wait_for_screen_marker(output_rx, output, "› /model", Duration::from_secs(10));
    writer
        .write_all(b"\x1b[A")
        .expect("wrap slash selection to last command");
    writer.flush().unwrap();
    wait_for_screen_marker(output_rx, output, "› /vim", Duration::from_secs(10));
    writer
        .write_all(b"\x1b")
        .expect("cancel slash suggestions without submitting draft");
    writer.flush().unwrap();
    wait_for_screen(
        output_rx,
        output,
        "slash popup closed with draft intact",
        |screen| !screen.contains("/model") && screen.lines().any(|line| line.trim() == "› /"),
    );
    clear_draft(writer, output_rx, output);

    write_typed_text(writer, b"@parser");
    let files = wait_for_screen(
        output_rx,
        output,
        "distinct filenames in search results",
        |screen| {
            screen.contains("› @parser")
                && screen.contains("parser_alpha.rs")
                && screen.contains("parser_beta.rs")
                && selected_filename(screen).is_some()
        },
    );
    let selected_before = selected_filename(&files)
        .unwrap_or_else(|| panic!("selected search row missing; screen: {files}"));
    writer
        .write_all(b"\x1b[B")
        .expect("navigate real file search results");
    writer.flush().unwrap();
    let files = wait_for_screen(output_rx, output, "file selection moved", |screen| {
        selected_filename(screen).is_some_and(|name| name != selected_before)
    });
    let selected = selected_filename(&files).unwrap();
    writer
        .write_all(b"\t")
        .expect("insert canonical file path with Tab");
    writer.flush().unwrap();
    let full_path = format!(
        "{}/{selected}",
        "long_directory_for_filename_identity_".repeat(3)
    );
    let inserted = wait_for_screen(
        output_rx,
        output,
        "original untruncated file path inserted",
        |screen| {
            screen
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>()
                .contains(&full_path)
        },
    );
    assert!(
        !inserted.contains(if selected == "parser_alpha.rs" {
            "parser_beta.rs"
        } else {
            "parser_alpha.rs"
        }),
        "file popup stayed visible after insertion: {inserted}"
    );
    clear_draft(writer, output_rx, output);

    write_typed_text(writer, b"$gate-skill-");
    wait_for_screen_marker(output_rx, output, "› $gate-skill-", Duration::from_secs(10));
    wait_for_screen_marker(
        output_rx,
        output,
        "› gate-skill-00",
        Duration::from_secs(10),
    );
    wait_for_screen_marker(output_rx, output, "[Skill]", Duration::from_secs(10));
    wait_for_screen_marker(
        output_rx,
        output,
        "enter insert · esc close",
        Duration::from_secs(10),
    );
    writer
        .write_all(b"\x1b[A")
        .expect("scroll skill suggestions to last catalog entry");
    writer.flush().unwrap();
    wait_for_screen_marker(
        output_rx,
        output,
        "› gate-skill-09",
        Duration::from_secs(10),
    );
    writer
        .write_all(b"\t")
        .expect("insert selected skill without starting a turn");
    writer.flush().unwrap();
    wait_for_screen(
        output_rx,
        output,
        "canonical skill token inserted and popup closed",
        |screen| screen.contains("$gate-skill-09") && !screen.contains("[Skill]"),
    );
    clear_draft(writer, output_rx, output);

    let ledger = std::fs::read_to_string(ledger_path).unwrap_or_default();
    assert!(
        !ledger
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .any(|entry| entry["scenario"] == "complete" && entry["kind"] == "turnStart"),
        "suggestion navigation/completion must not start a canonical turn"
    );
}

fn selected_filename(screen: &str) -> Option<&'static str> {
    screen
        .lines()
        .filter(|line| line.trim_start().starts_with('›'))
        .find_map(|line| {
            ["parser_alpha.rs", "parser_beta.rs"]
                .into_iter()
                .find(|name| line.contains(name))
        })
}

fn clear_draft(writer: &mut impl Write, output_rx: &mpsc::Receiver<Vec<u8>>, output: &mut String) {
    writer
        .write_all(b"\x15")
        .expect("clear completion draft with Ctrl-U");
    writer.flush().unwrap();
    wait_for_screen_marker(
        output_rx,
        output,
        "Ask Lime to do anything",
        Duration::from_secs(10),
    );
}
