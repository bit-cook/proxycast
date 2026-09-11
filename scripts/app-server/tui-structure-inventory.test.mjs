import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const inventory = JSON.parse(
  readFileSync(
    path.resolve(
      process.cwd(),
      "internal/exec-plans/tui-structure-inventory.json",
    ),
    "utf8",
  ),
);

describe("Codex TUI structure inventory", () => {
  it("records both TUI source trees", () => {
    expect(inventory.schemaVersion).toBe(1);
    expect(inventory.trees["codex-rs/tui/src"].fileCount).toBeGreaterThan(0);
    expect(inventory.trees["lime-rs/crates/tui/src"].fileCount).toBeGreaterThan(
      0,
    );
  });

  it("records the Codex-shaped TUI integration test trees", () => {
    for (const treeName of ["codex-rs/tui/tests", "lime-rs/crates/tui/tests"]) {
      expect(inventory.trees[treeName].fileCount).toBeGreaterThan(0);
    }
    expect(inventory.trees["lime-rs/crates/tui/tests"].files).toEqual(
      expect.arrayContaining([
        "all.rs",
        "test_backend.rs",
        "manager_dependency_regression.rs",
        "suite/mod.rs",
        "suite/vt100_history.rs",
        "suite/vt100_live_commit.rs",
        "suite/status_indicator.rs",
        "suite/focus_palette.rs",
        "suite/reconnect.rs",
        "suite/resize_reflow.rs",
      ]),
    );
    expect(inventory.comparisons.testFilesMissingInLime).toEqual(
      expect.arrayContaining(["fixtures/oss-story.jsonl"]),
    );
  });

  it("locks Codex-shaped current TUI module and symbol names", () => {
    const files = new Set(inventory.trees["lime-rs/crates/tui/src"].files);
    for (const file of [
      "markdown_render.rs",
      "status_indicator_widget.rs",
      "resume_picker.rs",
      "resume_picker/archive.rs",
      "resume_picker/archive_tests.rs",
      "resume_picker/page_loading.rs",
      "resume_picker_transcript_preview.rs",
      "resume_picker_transcript_preview_tests.rs",
      "bottom_pane/chat_composer.rs",
      "bottom_pane/chat_composer/attachment_state.rs",
      "bottom_pane/chat_composer/draft_state.rs",
      "bottom_pane/chat_composer/history_search.rs",
      "bottom_pane/chat_composer/reconnect.rs",
      "bottom_pane/chat_composer/reconnect_tests.rs",
      "bottom_pane/approval_overlay.rs",
      "bottom_pane/request_user_input/mod.rs",
      "bottom_pane/request_user_input/render.rs",
      "clipboard_copy.rs",
      "clipboard_paste.rs",
      "command_popup.rs",
      "model_catalog.rs",
      "collaboration_modes.rs",
      "app/app_server_events.rs",
      "app/app_server_requests.rs",
      "app/event_dispatch.rs",
      "app/input.rs",
      "app/reconnect.rs",
      "app/session_lifecycle.rs",
      "app/startup.rs",
      "app/startup_prompts.rs",
      "app/pending_interactive_replay.rs",
      "app/replay_filter.rs",
      "app/thread_events.rs",
      "app/thread_settings.rs",
      "app/tests.rs",
      "app/history_pagination.rs",
      "app/history_ui.rs",
      "app/transcript_export.rs",
      "app_server_session/history.rs",
      "app_server_session/history_tests.rs",
      "pending_input_preview.rs",
      "terminal_hyperlinks.rs",
      "reconnect.rs",
      "bottom_pane/textarea.rs",
      "bottom_pane/textarea/wrapping.rs",
      "bottom_pane/textarea/wrapping_tests.rs",
      "terminal_palette.rs",
      "table_detect.rs",
      "wrapping.rs",
      "render/mod.rs",
      "render/highlight.rs",
      "render/highlight_streaming.rs",
      "render/highlight_streaming_tests.rs",
      "render/line_utils.rs",
      "render/renderable.rs",
      "render/renderable_tests.rs",
      "cwd_prompt.rs",
      "insert_history.rs",
      "history_cell/mod.rs",
      "history_cell/base.rs",
      "history_cell/messages.rs",
      "history_cell/exec.rs",
      "history_cell/patches.rs",
      "history_cell/plans.rs",
      "history_cell/approvals.rs",
      "history_cell/mcp.rs",
      "history_cell/notices.rs",
      "history_cell/request_user_input.rs",
      "history_cell/separators.rs",
      "history_cell/session.rs",
      "exec_cell/mod.rs",
      "exec_cell/model.rs",
      "exec_cell/live_output.rs",
      "exec_cell/render.rs",
      "tui.rs",
      "tui/event_stream.rs",
      "tui/frame_rate_limiter.rs",
      "tui/frame_requester.rs",
      "selection_list.rs",
      "thread_transcript.rs",
      "transcript_reflow.rs",
    ]) {
      expect(files.has(file), file).toBe(true);
    }
    const symbols = new Set(
      inventory.trees["lime-rs/crates/tui/src"].symbols.map(
        (symbol) => symbol.name,
      ),
    );
    for (const name of [
      "render_markdown_text",
      "render_markdown_lines_with_width",
      "fmt_elapsed_compact",
      "ChatComposer",
      "InputResult",
      "AttachmentState",
      "DraftState",
      "TextArea",
      "TextAreaState",
      "input",
      "delete_backward",
      "delete_forward",
      "delete_forward_kill",
      "delete_backward_word",
      "delete_forward_word",
      "kill_to_end_of_line",
      "kill_to_beginning_of_line",
      "yank",
      "beginning_of_previous_word",
      "end_of_next_word",
      "handle_disconnected_key",
      "cursor_pos_with_state",
      "desired_height",
      "reconnect_session",
      "wrapped_lines",
      "cursor_position",
      "visible_prefix",
      "ApprovalOverlay",
      "Tui",
      "Terminal",
      "with_restored",
      "set_modes",
      "restore_keep_raw",
      "flush_terminal_input_buffer",
      "run_resume_picker_with_app_server",
      "run_fork_picker_with_app_server",
      "run_session_picker_with_app_server",
      "PickerState",
      "SessionTarget",
      "SessionSelection",
      "SessionPickerAction",
      "SessionPickerLaunchContext",
      "ArchiveState",
      "PaginationState",
      "load_transcript_preview",
      "load_session_transcript_with_handle",
      "thread_to_transcript_entries",
      "selection_option_row",
      "selection_option_row_with_dim",
      "TranscriptReflowState",
      "TranscriptWidthChange",
      "SessionTranscriptState",
      "ModelCatalog",
      "default_mask",
      "mask_for_kind",
      "next_mask",
      "default_mode_mask",
      "plan_mask",
      "to_mode",
      "handle_app_server_event",
      "handle_server_notification_event",
      "handle_server_request_event",
      "PendingInteractiveReplayState",
      "note_server_request",
      "note_server_notification",
      "should_replay_snapshot_request",
      "snapshot_has_pending_interactive_request",
      "event_is_notice",
      "omit_completed_agent_deltas",
      "handle_event",
      "EventContext",
      "EventDispatch",
      "handle_tui_event",
      "handle_key_event",
      "open_agent_picker",
      "render_expanded_session_details",
      "render_transcript_content_lines",
      "render_transcript_entry_lines",
      "render_transcript_entry_lines_wrapped",
      "EventBroker",
      "TuiEventStream",
      "TuiEvent",
      "FrameRequester",
      "RtOptions",
      "adaptive_wrap_line",
      "adaptive_wrap_lines",
      "word_wrap_line",
      "word_wrap_lines",
      "wrap_ranges",
      "wrap_ranges_trim",
      "url_preserving_wrap_options",
      "line_has_mixed_url_and_non_url_tokens",
      "parse_table_segments",
      "FenceTracker",
      "StdoutColorLevel",
      "best_color",
      "effective_stdout_color_level",
      "StreamingCodeHighlighter",
      "Renderable",
      "RenderableItem",
      "ColumnRenderable",
      "FlexRenderable",
      "RowRenderable",
      "InsetRenderable",
      "RenderableExt",
      "Insets",
      "RectExt",
      "line_to_borrowed",
      "line_to_static",
      "push_owned_lines",
      "prefix_lines",
      "HistoryLineWrapPolicy",
      "InsertHistoryMode",
      "insert_history_hyperlink_lines_with_mode_and_wrap_policy",
      "insert_history_lines",
      "insert_history_lines_with_mode_and_wrap_policy",
      "insert_history_lines_with_wrap_policy",
      "wrap_history_hyperlink_lines",
      "leading_whitespace_prefix",
      "HistoryCell",
      "TranscriptHistoryCell",
      "HistoryRenderMode",
      "PlainHistoryCell",
      "CompositeHistoryCell",
      "CommandOutput",
      "LiveCommandOutput",
      "output_lines",
      "ThreadHistoryPagination",
      "thread_items_page_params",
      "hydrate_initial_thread_history",
      "request_older_history_page",
      "handle_older_history_page",
      "render_markdown_transcript",
      "write_transcript",
      "StartupSessionState",
      "initialize_session",
      "should_wait_for_initial_session",
      "should_handle_active_thread_events",
      "should_stop_waiting_for_initial_session",
      "SkillLoadWarningState",
      "StartupTooltipOverride",
      "should_show_model_migration_prompt",
      "target_preset_for_upgrade",
      "apply_accepted_model_migration",
      "select_model_availability_nux",
      "CwdPromptAction",
      "CwdSelection",
      "CwdPromptOutcome",
      "set_model_catalog",
    ]) {
      expect(symbols.has(name), name).toBe(true);
    }
    expect(files.has("composer.rs")).toBe(false);
    expect(files.has("bottom_pane/request_user_input.rs")).toBe(false);
    expect(files.has("terminal.rs")).toBe(false);
    expect(symbols.has("TerminalGuard")).toBe(false);
    expect(symbols.has("TuiTerminal")).toBe(false);
  });

  it("locks direct snapshot test names to the Codex baseline", () => {
    const expected = {
      "lime-rs/crates/tui/src/diff_render.rs": [
        "add_details",
        "ansi16_insert_delete_no_background",
        "apply_add_block",
        "apply_delete_block",
        "apply_multiple_files_block",
        "apply_update_block",
        "apply_update_block_line_numbers_three_digits_text",
        "apply_update_block_relativizes_path",
        "apply_update_block_wraps_long_lines",
        "apply_update_block_wraps_long_lines_text",
        "apply_update_with_rename_block",
        "blank_context_line",
        "cpp_module_extension_highlighting",
        "diff_gallery_120x40",
        "diff_gallery_80x24",
        "diff_gallery_94x35",
        "single_line_replacement_counts",
        "syntax_highlighted_insert_wraps",
        "syntax_highlighted_insert_wraps_text",
        "theme_scope_background_resolution",
        "update_details_with_rename",
        "vertical_ellipsis_between_hunks",
        "wrap_behavior_insert",
      ],
      "lime-rs/crates/tui/src/markdown.rs": [
        "label_only_and_fallback_presentations_snapshot",
        "bare_url_with_tilde_keeps_complete_hyperlink",
        "file_link_compares_path_spellings_without_changing_display",
        "file_link_ignores_trailing_separators_when_comparing_paths",
        "file_link_keeps_descriptive_label_and_target",
        "file_link_keeps_unrelated_relative_label_with_matching_suffix",
        "file_link_preserves_labels_with_invalid_percent_encoding",
        "file_link_preserves_tilde_and_absolute_destinations",
        "list_item_after_code_block_keeps_blank_separator",
        "markdown_render_complex_snapshot",
        "markdown_render_file_link_snapshot",
        "mixed_url_markdown_wraps_prose_without_splitting_words_snapshot",
        "multiline_finding_items_are_separated_snapshot",
        "table_keeps_grid_when_only_one_compact_record_fragments_snapshot",
        "table_renders_halfwidth_sound_marks_at_constrained_width_snapshot",
        "table_renders_key_value_records_when_compact_fragmentation_is_systemic_snapshot",
        "table_renders_records_when_multiple_prose_columns_are_starved_snapshot",
        "table_renders_stacked_key_value_records_when_path_column_becomes_too_narrow_snapshot",
        "table_wraps_file_paths_before_collapsing_narrative_columns_snapshot",
        "web_link_labels_have_a_visible_underline_snapshot",
      ],
      "lime-rs/crates/tui/src/terminal_hyperlinks.rs": [
        "buffer_hyperlinks_follow_scrolled_wrapped_rows",
        "forced_width_hyperlinks_render_wide_and_halfwidth_cells_snapshot",
      ],
      "lime-rs/crates/tui/src/insert_history.rs": [
        "vt100_zellij_raw_insert_keeps_soft_wrapped_tail_above_viewport",
        "vt100_zellij_raw_replay_keeps_overflowing_soft_wrapped_tail_above_viewport",
      ],
      "lime-rs/crates/tui/src/render/highlight.rs": [
        "ansi_family_foreground_palette",
      ],
    };
    for (const [file, names] of Object.entries(expected)) {
      const source = readFileSync(path.resolve(process.cwd(), file), "utf8");
      const actual = new Set(
        [
          ...source.matchAll(
            /^[ \t]*(?:pub\([^)]*\)[ \t]*)?fn[ \t]+([a-z][a-z0-9_]*)[ \t]*\(/gmu,
          ),
        ].map((match) => match[1]),
      );
      for (const name of names) {
        expect(actual.has(name), `${file}:${name}`).toBe(true);
      }
    }
  });

  it("keeps app-server event routing in the Codex-named owner", () => {
    const appServerEvents = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/app_server_events.rs",
      ),
      "utf8",
    );
    const appServerRequests = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/app_server_requests.rs",
      ),
      "utf8",
    );
    const runtime = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime.rs"),
      "utf8",
    );
    const appServerClient = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/app-server-client/src/lib.rs",
      ),
      "utf8",
    );
    const interactiveRuntime = runtime.slice(
      0,
      runtime.indexOf("pub async fn run_exec"),
    );

    expect(appServerEvents).toContain("fn handle_app_server_event");
    expect(appServerEvents).toContain("fn handle_server_notification_event");
    expect(appServerEvents).not.toContain("fn handle_server_request_event");
    expect(appServerRequests).toContain("fn handle_server_request_event");
    expect(runtime).toContain("app.handle_app_server_event(");
    expect(interactiveRuntime).not.toContain(
      "AppServerEvent::ServerNotification",
    );
    expect(interactiveRuntime).not.toContain("AppServerEvent::ServerRequest");
    expect(appServerClient).toContain("pub enum AppServerEvent");
    expect(appServerClient).not.toContain("pub enum SessionEvent");
    expect(appServerClient).not.toContain("RawNotification(");
    expect(appServerClient).not.toContain("RawServerRequest(");
  });

  it("keeps replay filtering bounded by the Lime protocol", () => {
    const replayFilter = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/replay_filter.rs",
      ),
      "utf8",
    );
    expect(replayFilter).toContain("snapshot_has_pending_interactive_request");
    expect(replayFilter).toContain("event_is_notice");
    expect(replayFilter).toContain("omit_completed_agent_deltas");
    expect(replayFilter).not.toContain("omit_resolved_misalignment_errors");
  });

  it("keeps App Server-backed action dispatch in the Codex-named owner", () => {
    const eventDispatch = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/event_dispatch.rs",
      ),
      "utf8",
    );
    const runtime = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime.rs"),
      "utf8",
    );

    expect(eventDispatch).toContain("pub(crate) async fn handle_event");
    expect(eventDispatch).toContain("pub(crate) struct EventContext");
    expect(runtime).toContain(".handle_event(");
    expect(runtime).not.toContain("AppAction::SelectModel(selection) =>");
    expect(runtime).not.toContain("AppAction::RefreshAgentsOverview =>");
  });

  it("keeps terminal input routing in the Codex-named owners", () => {
    const app = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/app.rs"),
      "utf8",
    );
    const input = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/app/input.rs"),
      "utf8",
    );
    const runtime = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime.rs"),
      "utf8",
    );

    expect(app).toContain("mod input;");
    expect(app).toContain("mod tests;");
    expect(app).toContain("fn handle_tui_event");
    expect(input).toContain("fn handle_key_event");
    expect(runtime).toContain("app.handle_tui_event(event, connected)");
    expect(runtime).not.toContain("handle_terminal_event");
    expect(runtime).not.toContain("handle_disconnected_event");
  });

  it("keeps reconnect lifecycle in the Codex-named app owner", () => {
    const appReconnect = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/app/reconnect.rs"),
      "utf8",
    );
    const runtime = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/runtime.rs"),
      "utf8",
    );

    expect(appReconnect).toContain("fn reconnect_session");
    expect(runtime).toContain("crate::app::reconnect::");
    expect(runtime).toContain("reconnect_session");
    expect(runtime).toContain("ReconnectedSession");
  });

  it("keeps thread notification projection in the Codex-named owner", () => {
    const threadEvents = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/thread_events.rs",
      ),
      "utf8",
    );
    const appServerEvents = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/app_server_events.rs",
      ),
      "utf8",
    );

    expect(threadEvents).toContain("fn apply_notification");
    expect(threadEvents).toContain("fn observe_notification");
    expect(threadEvents).toContain("fn observe_item");
    expect(appServerEvents).toContain("self.apply_notification(notification)");
    expect(appServerEvents).not.toContain("fn observe_notification");
  });

  it("keeps foreign interactive requests in ThreadEventStore only", () => {
    const agentsOverview = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/agents_overview.rs",
      ),
      "utf8",
    );
    const threadEvents = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/thread_events.rs",
      ),
      "utf8",
    );
    const pendingReplay = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/pending_interactive_replay.rs",
      ),
      "utf8",
    );

    expect(agentsOverview).not.toContain("dispatched_requests");
    expect(agentsOverview).not.toContain("queue_agents_overview_request");
    expect(threadEvents).toContain("ThreadEventStore");
    expect(pendingReplay).toContain("PendingInteractiveReplayState");
  });

  it("keeps thread settings state in the Codex-named owner", () => {
    const threadSettings = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/thread_settings.rs",
      ),
      "utf8",
    );
    const app = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/app.rs"),
      "utf8",
    );

    expect(threadSettings).toContain("fn set_settings");
    expect(threadSettings).toContain("fn set_permission_profiles");
    expect(threadSettings).toContain("fn next_collaboration_mode");
    expect(app).not.toContain("fn sync_default_collaboration_mode");
  });

  it("keeps agent picker lifecycle in the Codex-named owner", () => {
    const app = readFileSync(
      path.resolve(process.cwd(), "lime-rs/crates/tui/src/app.rs"),
      "utf8",
    );
    const sessionLifecycle = readFileSync(
      path.resolve(
        process.cwd(),
        "lime-rs/crates/tui/src/app/session_lifecycle.rs",
      ),
      "utf8",
    );

    expect(app).toContain("mod session_lifecycle;");
    expect(sessionLifecycle).toContain("fn open_agent_picker");
  });

  it("keeps upstream product-only differences explicit", () => {
    expect(inventory.comparisons.filesMissingInLime).toContain(
      "onboarding/mod.rs",
    );
    expect(
      inventory.comparisons.symbolNamesMissingInLime.length,
    ).toBeGreaterThan(0);
    expect(inventory.comparisons.filesOnlyInLime).not.toContain(
      "session_picker.rs",
    );
  });
});
