import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import YAML from "yaml";

const gateSource = readFileSync(
  path.resolve(process.cwd(), "scripts/app-server/tui-gate-b.mjs"),
  "utf8",
);
const runtimeSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime.rs"),
  "utf8",
);
const ptyTestSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime_pty_tests.rs"),
  "utf8",
);
const terminalFixtureSource = readFileSync(
  path.resolve(process.cwd(), "scripts/app-server/terminal-gate-fixture.mjs"),
  "utf8",
);
const suggestionTestSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime_pty_tests/suggestions.rs"),
  "utf8",
);
const approvalTestSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime_pty_tests/approval.rs"),
  "utf8",
);
const focusTestSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/tests/suite/focus_palette.rs"),
  "utf8",
);
const resizeTestSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/tests/suite/resize_reflow.rs"),
  "utf8",
);
const reconnectTestSource = readFileSync(
  path.resolve(process.cwd(), "lime-rs/crates/tui/tests/suite/reconnect.rs"),
  "utf8",
);

describe("TUI Gate B", () => {
  it("views approval details without resolving the protected request", () => {
    expect(ptyTestSource).toContain("approval::exercise_read_only_details");
    expect(approvalTestSource).toContain('"open approval details with Ctrl-A"');
    expect(approvalTestSource).toContain('"close details without deciding approval"');
    expect(approvalTestSource).toContain('"closing approval details must not resolve the canonical request"');
  });
  it("drives catalog-backed suggestions through real keyboard completion", () => {
    expect(gateSource).toContain('"parser_alpha.rs", "parser_beta.rs"');
    expect(gateSource).toContain('".agents", "skills", name');
    expect(ptyTestSource).toContain("suggestions::exercise_suggestion_menus");
    expect(suggestionTestSource).toContain('"original untruncated file path inserted"');
    expect(suggestionTestSource).toContain('"› gate-skill-09"');
    expect(suggestionTestSource).toContain('"canonical skill token inserted and popup closed"');
    expect(suggestionTestSource).toContain('"suggestion navigation/completion must not start a canonical turn"');
    expect(suggestionTestSource).not.toContain("thread::sleep");
  });
  it("drives the real TUI through a portable PTY and current App Server", () => {
    expect(gateSource).toContain('LIME_TEST_TUI_GATE_B: "1"');
    expect(gateSource).toContain("buildTerminalGateBinaries");
    expect(gateSource).toContain("LIME_TEST_TERMINAL_CWD: scenarioDir");
    expect(gateSource).toContain('"--exact"');
    expect(gateSource).toContain("writeTerminalExternalBackend");
    expect(ptyTestSource).toContain('OsString::from("tui")');
    expect(gateSource).toContain(
      "runtime::pty_tests::real_pty_restores_terminal_after_visible_turn_completion",
    );
    expect(gateSource).toContain(
      "suite::focus_palette::focus_gained_with_unanswered_palette_queries_preserves_immediate_input",
    );
    expect(gateSource).toContain('"--test",\n        "all"');
    expect(gateSource).toContain('"focus-palette=ok"');
    expect(focusTestSource).toContain(
      "focus_gained_with_unanswered_palette_queries_preserves_immediate_input",
    );
    expect(focusTestSource).toContain('b"\\x1b[I"');
    expect(focusTestSource).toContain(
      "focus regain queried terminal colors after startup palette was cached",
    );
    expect(focusTestSource).toContain('b"\\x1b[?1049l"');
    expect(gateSource).toContain('"suite::resize_reflow::"');
    expect(gateSource).toContain('"--test-threads=1"');
    expect(gateSource).toContain('"resize-reflow=ok"');
    expect(resizeTestSource).toContain(
      "tmux_split_preserves_fresh_session_composer_row_after_resize_reflow",
    );
    expect(resizeTestSource).toContain("tmux_repeated_resizes_do_not_push_composer_down");
    expect(resizeTestSource).toContain(
      "tmux_width_resize_restore_keeps_visible_content_anchored",
    );
    expect(resizeTestSource).toContain(
      "tmux_scrolled_composer_resize_preserves_visible_draft_text",
    );
    expect(resizeTestSource).toContain("terminal.resize(");
    expect(focusTestSource).toContain("self.master.resize");
    expect(gateSource).toContain(
      "suite::reconnect::automatic_reconnect_restores_draft_and_routes_new_notifications",
    );
    expect(gateSource).toContain('"reconnect=ok"');
    expect(reconnectTestSource).toContain(
      "automatic_reconnect_restores_draft_and_routes_new_notifications",
    );
    expect(reconnectTestSource).toContain('OsString::from("--remote")');
    expect(reconnectTestSource).toContain("thread/resume");
    expect(reconnectTestSource).toContain('"config/read"');
    expect(reconnectTestSource).toContain(
      "TUI settings must be read exactly once into the startup snapshot",
    );
    expect(reconnectTestSource).toContain("fresh-notification-after-reconnect");
    expect(reconnectTestSource).toContain("preserved-draft!");
    expect(reconnectTestSource).toContain('b"\\x1b[?1049l"');
    expect(ptyTestSource).toContain("native_pty_system()");
    expect(ptyTestSource).toContain('output.contains("\\u{1b}[?1049h")');
    expect(ptyTestSource).toContain('output.contains("\\u{1b}[?1049l")');
    expect(ptyTestSource).toContain("EDITOR_JOB_CONTROL_OK");
    expect(ptyTestSource).toContain("configure_external_editor");
    expect(ptyTestSource).toContain('"open shortcut overlay"');
    expect(ptyTestSource).toContain('"Keyboard shortcuts"');
    expect(ptyTestSource).toContain('"? / esc close"');
    expect(ptyTestSource).toContain('"close shortcut overlay without interrupt"');
    expect(ptyTestSource).toContain('"close active help without cancelling turn"');
    expect(ptyTestSource).toContain('"closing shortcut help must not interrupt canonical turn"');
    expect(ptyTestSource).toContain('"Select model"');
    expect(ptyTestSource).toContain('"cancel model picker without changing settings"');
    expect(ptyTestSource).not.toContain('write_all(b"\\x1b[1;1R")');
    expect(ptyTestSource).toContain("writer.write_all(&[20])");
    expect(ptyTestSource).toContain('"ctrl+t·esc·q close"');
    expect(ptyTestSource).toContain(
      '"drag main transcript selection with SGR mouse input"',
    );
    expect(ptyTestSource).toContain(
      '"main transcript SGR mouse selection was not visible"',
    );
    expect(ptyTestSource).toContain('"sticky prompt header position"');
    expect(ptyTestSource).toContain(
      '"main transcript selection displaced the sticky prompt header"',
    );
    expect(ptyTestSource).toContain(
      '"compact transcript sticky prompt header was not visible"',
    );
    expect(gateSource).toContain('"sticky-prompt=ok"');
    expect(gateSource).toContain('"      find_transcript: ctrl-x f"');
    expect(ptyTestSource).toContain('write_all(b"\\x18")');
    expect(ptyTestSource).toContain(
      '"start configured main transcript Find chord with Ctrl-X"',
    );
    expect(ptyTestSource).toContain(
      '"complete configured main transcript Find chord"',
    );
    expect(ptyTestSource).toContain(
      '"configured compact transcript Find chord and match highlight were not visible"',
    );
    expect(gateSource).toContain('"main-find=ok"');
    expect(ptyTestSource).toContain(
      '"drag transcript selection with SGR mouse input"',
    );
    expect(ptyTestSource).toContain("wait_for_inverse_cells");
    expect(ptyTestSource).toContain(
      '"transcript SGR mouse selection was not visible"',
    );
    expect(ptyTestSource).toContain('write_all(b"\\0\\x1b[C")');
    expect(ptyTestSource).toContain(
      '"transcript Ctrl-Space keyboard selection was not visible"',
    );
    expect(gateSource).toContain("scrollableCompletedText");
    expect(ptyTestSource).toContain(
      '"hold a vertical transcript selection at the bottom edge"',
    );
    expect(ptyTestSource).toContain(
      '"transcript edge drag did not continuously scroll the canonical projection"',
    );
    expect(ptyTestSource).toContain('"bookmark transcript at top"');
    expect(ptyTestSource).toContain('"reopen transcript Ctrl-T"');
    expect(ptyTestSource).toContain(
      '"detailed transcript bookmark was not restored after Ctrl-T reopen"',
    );
    expect(ptyTestSource).toContain('writer.write_all(b"\\x1bOS")');
    expect(ptyTestSource).toContain('"+ Show details"');
    expect(ptyTestSource).toContain('"− Show less"');
    expect(ptyTestSource).toContain(
      '"transcript activity disclosure was not visible"',
    );
    expect(terminalFixtureSource).toContain('kind: "reasoning"');
    expect(terminalFixtureSource).toContain('type: "reasoning"');
    expect(gateSource).toContain("LIME_TEST_TERMINAL_REASONING_TEXT");
    expect(gateSource).toContain("LIME_TEST_TERMINAL_RAW_TEXT");
    expect(ptyTestSource).toContain(
      '"transcript-only reasoning leaked into the compact main transcript"',
    );
    expect(ptyTestSource).toContain(
      '"transcript-only reasoning was not retained in the detailed transcript"',
    );
    expect(ptyTestSource).toContain('write_all(b"\\x1br")');
    expect(ptyTestSource).toContain(
      '"Alt-R raw output did not expose canonical markdown source"',
    );
    expect(ptyTestSource).toContain('write_all(b"\\x1b[5~")');
    expect(ptyTestSource).toContain('"Back to bottom"');
    expect(ptyTestSource).toContain(
      '"compact transcript return-to-latest control was not visible"',
    );
    expect(ptyTestSource).toContain('writer.write_all(b"\\x1b")');
    expect(ptyTestSource).toContain('"esc to interrupt"');
    expect(ptyTestSource).toContain('write_all(b"\\x1b[1;3A")');
    expect(ptyTestSource).toContain('"editing queued"');
    expect(ptyTestSource).toContain('write_typed_text(&mut writer, b"/agents\\r")');
    expect(ptyTestSource).toContain('write_all(b"n")');
    expect(ptyTestSource).toContain('write_all(b"r")');
    expect(ptyTestSource).toContain('write_all(b"x")');
    expect(ptyTestSource).toContain('write_all(b"\\t\\t")');
    expect(ptyTestSource).toContain('"Rename ›"');
    expect(ptyTestSource).toContain('"Search ›"');
    expect(ptyTestSource).toContain('"Agent command center"');
    expect(ptyTestSource).toContain("vt100::Parser::new(24, 100, 0)");
    expect(ptyTestSource).toContain('"background task started"');
    expect(ptyTestSource).toContain('"Working 1"');
    expect(ptyTestSource).toContain('"Gate B background"');
    expect(ptyTestSource).toContain('"AGENTS_OVERVIEW_READY"');
    expect(gateSource).toContain('event?.type === "queue.added"');
    expect(gateSource).toContain('event?.type === "queue.removed"');
    expect(gateSource).toContain(
      'event.payload?.source === "thread/queue/delete"',
    );
    expect(gateSource).toContain(
      '"complete,approval,user-input,interrupt,failure,queue-edit,agents-overview"',
    );
  });

  it("does not substitute a mock backend or synthetic completion event", () => {
    expect(ptyTestSource).toContain(
      'OsString::from("--app-server-arg=external")',
    );
    expect(gateSource).not.toContain('APP_SERVER_BACKEND_MODE: "mock"');
    expect(gateSource).not.toContain("turn.final_done");
    expect(runtimeSource).not.toContain("final_done");
    expect(runtimeSource).not.toContain("poll_crossterm_event");
  });

  it("keeps Windows CLI and TUI current-path evidence in the package workflow", () => {
    const workflow = YAML.parse(
      readFileSync(
        path.resolve(process.cwd(), ".github/workflows/build-windows-test.yml"),
        "utf8",
      ),
    );
    const steps = workflow.jobs["build-windows-test"].steps;
    const gate = steps.find(
      (step) => step.name === "Run Windows CLI and TUI current-path gates",
    );
    const upload = steps.find(
      (step) => step.name === "Upload Windows CLI and TUI Gate B evidence",
    );

    expect(gate?.shell).toBe("bash");
    expect(gate?.run).toContain(
      "cargo test --manifest-path lime-rs/Cargo.toml -p cli -p tui",
    );
    expect(gate?.run).toContain(
      "cargo build --manifest-path lime-rs/Cargo.toml -p cli -p app-server",
    );
    expect(gate?.run).toContain("npm run smoke:cli-gate-b");
    expect(gate?.run).toContain("npm run smoke:tui-gate-b");
    expect(gate?.run).toContain('} 2>&1 | tee "windows-cli-tui-gate-b.log"');
    expect(upload?.if).toBe("${{ always() }}");
    expect(upload?.with?.path).toBe("windows-cli-tui-gate-b.log");
    expect(upload?.with?.["if-no-files-found"]).toBe("error");
  });
});
