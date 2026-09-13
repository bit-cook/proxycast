use std::ffi::OsString;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use app_server_client::{AppServerEvent, RemoteTransportConfig, StdioTransportConfig};
use app_server_protocol::protocol::v2::{ServerNotification, ServerRequest, UserInput};
use futures::StreamExt;
use serde::Serialize;

use crate::app::event_dispatch::{EventContext, EventDispatch};
use crate::app::reconnect::{reconnect_session, ReconnectedSession};
use crate::app::{App, AppAction, ExternalEditorState};
use crate::app_server_session::AppServerSession;
use crate::bottom_pane::{AppServerResponse, FileSearchRequest};
use crate::clipboard_copy::copy_to_clipboard;
use crate::clipboard_paste::paste_image_to_temp_png;
use crate::external_editor::edit_draft;
use crate::locale::Locale;
use crate::projection::{ConversationProjection, TranscriptEntry};
use crate::resume_picker::{
    run_resume_picker_with_app_server, PickerAction, PickerLoadEvent, PickerState,
    SessionPickerAction, SessionStatus,
};
use crate::settings::{parse_settings_command, SettingsCommand};
use crate::tui::{Tui, TuiEvent};
use crate::view;

#[derive(Debug, Clone)]
pub struct TuiOptions {
    pub app_server_bin: PathBuf,
    pub app_server_args: Vec<OsString>,
    pub remote: Option<RemoteTransportConfig>,
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub reasoning_effort: Option<String>,
    pub permissions: Option<String>,
    pub locale: Option<String>,
    pub resume_thread: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecOptions {
    pub tui: TuiOptions,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecResult {
    pub thread_id: String,
    pub turn_id: String,
    pub status: String,
    pub output: String,
}

#[derive(Debug)]
struct FileSearchEvent {
    generation: u64,
    query: String,
    files: Vec<app_server_protocol::protocol::v2::FuzzyFileSearchResult>,
}

fn validate_model_route(options: &TuiOptions) -> Result<()> {
    match (&options.model, &options.model_provider) {
        (Some(model), Some(provider)) if model.trim().is_empty() || provider.trim().is_empty() => {
            bail!("model and provider must not be empty")
        }
        (Some(_), None) | (None, Some(_)) => {
            bail!("--model and --provider must be specified together")
        }
        _ => Ok(()),
    }
}

fn spawn_file_search(
    request_handle: app_server_client::RequestHandle,
    cwd: PathBuf,
    request: FileSearchRequest,
    result_tx: tokio::sync::mpsc::UnboundedSender<FileSearchEvent>,
) {
    tokio::spawn(async move {
        match AppServerSession::fuzzy_file_search_request(
            request_handle,
            cwd,
            request.query.clone(),
        )
        .await
        {
            Ok(files) => {
                let _ = result_tx.send(FileSearchEvent {
                    generation: request.generation,
                    query: request.query,
                    files,
                });
            }
            Err(error) => {
                tracing::debug!("file search request failed: {error}");
            }
        }
    });
}

pub async fn run_tui(options: TuiOptions) -> Result<()> {
    validate_model_route(&options)?;
    let mut session = Some(connect_session(&options).await?);
    let mut app = App::default();
    app.set_cwd(options.cwd.clone());
    app.set_locale(Locale::resolve(options.locale.as_deref()));
    app.begin_startup_input_boundary();
    let setup_result = crate::app::startup::initialize_session(
        &options,
        session.as_mut().expect("session available during setup"),
        &mut app,
    )
    .await;
    let (mut model, mut model_provider, mut effort, mut permissions) = match setup_result {
        Ok(state) => (
            state.model,
            state.model_provider,
            state.effort,
            state.permissions,
        ),
        Err(error) => {
            let _ = session
                .take()
                .expect("session available after setup failure")
                .shutdown()
                .await;
            return Err(error);
        }
    };
    let mut terminal = match Tui::enter().context("failed to initialize terminal") {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = session
                .take()
                .expect("session available before terminal setup")
                .shutdown()
                .await;
            return Err(error);
        }
    };
    let mut input = terminal.event_stream();
    let frame_requester = terminal.frame_requester();
    frame_requester.schedule_frame();
    let mut status_tick = tokio::time::interval(Duration::from_secs(1));
    status_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let run_result: Result<()> = async {
        let mut resume_picker_load_tx: Option<tokio::sync::mpsc::UnboundedSender<PickerLoadEvent>> =
            None;
        let mut resume_picker_load_rx: Option<tokio::sync::mpsc::UnboundedReceiver<PickerLoadEvent>> =
            None;
        let mut reconnect: Option<Pin<Box<dyn Future<Output = Result<ReconnectedSession>>>>> = None;
        let mut reconnect_thread_id: Option<String> = None;
        let mut reconnect_failed = false;
        let mut pending_tui_event: Option<TuiEvent> = None;
        let (file_search_tx, mut file_search_rx) =
            tokio::sync::mpsc::unbounded_channel::<FileSearchEvent>();
        loop {
            if session.is_none() && reconnect.is_none() && !reconnect_failed {
                if let Some(thread_id) = reconnect_thread_id.as_deref() {
                    reconnect = Some(Box::pin(reconnect_session(
                        options.clone(),
                        thread_id.to_string(),
                        model.clone(),
                        model_provider.clone(),
                        effort.clone(),
                        permissions.clone(),
                    )));
                } else {
                    reconnect_failed = true;
                    app.projection
                        .set_status("reconnect unavailable: thread id missing");
                }
            }
            terminal
                .sync_viewport()
                .context("failed to synchronize terminal viewport")?;
            if let Some(active_session) = session.as_ref() {
                if let (Some(picker), Some(sender)) =
                    (app.resume_picker.as_mut(), resume_picker_load_tx.as_ref())
                {
                    if let Some(thread_id) = picker.selected_thread_id().map(ToOwned::to_owned) {
                        crate::resume_picker::spawn_preview_load(
                            active_session.request_handle(),
                            sender,
                            picker,
                            thread_id,
                        );
                    }
                }
            }
            terminal
                .terminal_mut()
                .draw(|frame| view::render(frame, &app))
                .context("failed to render terminal")?;

            if app.external_editor_state() == ExternalEditorState::Requested {
                app.set_external_editor_state(ExternalEditorState::Active);
                let draft = app.composer.text().to_string();
                let edited = terminal
                    .with_restored(|| edit_draft(&draft, &options.cwd))
                    .await;
                app.reset_external_editor_state();
                match edited {
                    Ok(Some(text)) => app.replace_composer(text),
                    Ok(None) => app.projection.set_status("editor draft empty"),
                    Err(error) => app.projection.set_status(error.to_string()),
                }
                let mut context = std::task::Context::from_waker(std::task::Waker::noop());
                if let std::task::Poll::Ready(event) = input.poll_crossterm_event(&mut context) {
                    pending_tui_event = event;
                }
                continue;
            }

            tokio::select! {
                file_search_event = file_search_rx.recv() => {
                    if let Some(file_search_event) = file_search_event {
                        app.composer.on_file_search_result(
                            file_search_event.generation,
                            &file_search_event.query,
                            file_search_event.files,
                        );
                        frame_requester.schedule_frame();
                    }
                }
                resume_event = async {
                    match resume_picker_load_rx.as_mut() {
                        Some(receiver) => receiver.recv().await,
                        None => std::future::pending::<Option<PickerLoadEvent>>().await,
                    }
                }, if resume_picker_load_rx.is_some() => {
                    let Some(resume_event) = resume_event else {
                        resume_picker_load_rx = None;
                        resume_picker_load_tx = None;
                        continue;
                    };
                    let Some(picker) = app.resume_picker.as_mut() else {
                        continue;
                    };
                    match resume_event {
                        PickerLoadEvent::Threads { token, result } => match result {
                            Ok(page) => {
                                picker.apply_thread_page(token, page);
                                if picker.threads.is_empty() && picker.has_more_pages() {
                                    if let Some(sender) = resume_picker_load_tx.as_ref() {
                                        crate::resume_picker::spawn_thread_load(
                                            session
                                                .as_ref()
                                                .expect("session available during resume picker")
                                                .request_handle(),
                                            sender,
                                            picker,
                                        );
                                    }
                                }
                            }
                            Err(error) if picker.load_token == token => {
                                picker.fail_thread_load(token, error);
                            }
                            Err(_) => {}
                        },
                        PickerLoadEvent::Preview { thread_id, result } => {
                            picker.set_transcript_preview(thread_id, result.unwrap_or_default());
                        }
                        PickerLoadEvent::Transcript { thread_id, result } => {
                            picker.set_transcript(thread_id, result);
                        }
                        PickerLoadEvent::Archive { thread_id, result } => {
                            picker.handle_archive_result(thread_id, result);
                        }
                        PickerLoadEvent::Unarchive { thread_id, result } => {
                            if let Some(crate::resume_picker::SessionSelection::Resume(target)) =
                                picker.handle_unarchive_result(thread_id, *result)
                            {
                                let thread_id = target.thread_id;
                                app.resume_picker = None;
                                resume_picker_load_rx = None;
                                resume_picker_load_tx = None;
                                app.agents_overview = None;
                                match app.resume_target_session(
                                    session
                                        .as_mut()
                                        .expect("session available during resume picker"),
                                    thread_id,
                                    &mut model,
                                    &mut model_provider,
                                    &mut effort,
                                    &mut permissions,
                                    &options,
                                )
                                .await
                                {
                                    Ok(()) => app.projection.set_status("archived session restored"),
                                    Err(error) => app
                                        .projection
                                        .set_status(format!("resume failed: {error}")),
                                }
                            }
                        }
                    }
                }
                _ = status_tick.tick(), if app.projection.active_turn_id().is_some() => {
                    frame_requester.schedule_frame();
                }
                event = async {
                    match pending_tui_event.clone() {
                        Some(event) => Some(event),
                        None => input.next().await,
                    }
                } => {
                    pending_tui_event = None;
                    let Some(event) = event else { break };
                    let event = match event {
                        TuiEvent::Draw => {
                            if app.projection.active_turn_id().is_some() {
                                frame_requester.schedule_frame_in(Duration::from_secs(1));
                            }
                            continue;
                        }
                        TuiEvent::Resize(size) => {
                            terminal.update_viewport(size, size.height);
                            TuiEvent::Resize(size)
                        }
                        TuiEvent::Resume => continue,
                        event => event,
                    };
                    let connected = session.is_some();
                    let action = app.handle_tui_event(event, connected);
                    if connected {
                        if let Some(request) = app.composer.take_file_search_request() {
                            spawn_file_search(
                                session
                                    .as_ref()
                                    .expect("session available for file search")
                                    .request_handle(),
                                app.cwd.clone(),
                                request,
                                file_search_tx.clone(),
                            );
                        }
                    }
                    if !connected {
                        if matches!(action, AppAction::Quit) {
                            break;
                        }
                        continue;
                    }
                    let action = match app
                        .handle_event(
                            action,
                            EventContext {
                                session: session
                                    .as_mut()
                                    .expect("session available during TUI action dispatch"),
                                model: &mut model,
                                model_provider: &mut model_provider,
                                effort: &mut effort,
                                permissions: &mut permissions,
                            },
                        )
                        .await?
                    {
                        EventDispatch::Handled => continue,
                        EventDispatch::Unhandled(action) => action,
                    };
                    match action {
                        AppAction::Submit(prompt) => {
                            if !app.can_accept_direct_input() {
                                continue;
                            }
                            if let Some(command) = parse_settings_command(&prompt) {
                                match command {
                                    Ok(SettingsCommand::Plan) => {
                                        let Some(collaboration_mode) = app.plan_mode() else {
                                            app.projection
                                                .set_status("plan mode unavailable on this server");
                                            continue;
                                        };
                                        match session
                                            .as_ref()
                                            .expect("session available during TUI")
                                            .update_collaboration_mode(collaboration_mode.clone())
                                            .await
                                        {
                                            Ok(()) => {
                                                model = Some(collaboration_mode.settings.model.clone());
                                                effort = collaboration_mode.settings.reasoning_effort.clone();
                                                app.set_settings(
                                                    model.clone(),
                                                    model_provider.clone(),
                                                    effort.clone(),
                                                    permissions.clone(),
                                                );
                                                app.collaboration_mode = Some(collaboration_mode);
                                                app.projection.set_status("plan mode");
                                            }
                                            Err(error) => app.projection.set_status(error.to_string()),
                                        }
                                    }
                                    Ok(SettingsCommand::ModelPicker) => {
                                        match session
                                            .as_ref()
                                            .expect("session available during TUI")
                                            .list_models(100)
                                            .await
                                        {
                                            Ok(response) => {
                                                app.open_model_picker(response.data);
                                                app.projection.set_status("choose model");
                                            }
                                            Err(error) => {
                                                app.projection.set_status(error.to_string());
                                            }
                                        }
                                    }
                                    Ok(SettingsCommand::Model {
                                        model: model_value,
                                        provider,
                                    }) => {
                                        match session
                                            .as_ref()
                                            .expect("session available during TUI")
                                            .update_settings(
                                                Some(model_value.clone()),
                                                provider.clone(),
                                                None,
                                                None,
                                            )
                                            .await
                                        {
                                            Ok(()) => {
                                                model = Some(model_value);
                                                model_provider = provider;
                                                app.set_settings(
                                                    model.clone(),
                                                    model_provider.clone(),
                                                    effort.clone(),
                                                    permissions.clone(),
                                                );
                                                app.projection.set_status("settings updated");
                                            }
                                            Err(error) => app.projection.set_status(error.to_string()),
                                        }
                                    }
                                    Ok(SettingsCommand::Effort(value)) => {
                                        match session
                                            .as_ref()
                                            .expect("session available during TUI")
                                            .update_settings(None, None, Some(value.clone()), None)
                                            .await
                                        {
                                            Ok(()) => {
                                                effort = Some(value);
                                                app.set_settings(
                                                    model.clone(),
                                                    model_provider.clone(),
                                                    effort.clone(),
                                                    permissions.clone(),
                                                );
                                                app.projection.set_status("settings updated");
                                            }
                                            Err(error) => app.projection.set_status(error.to_string()),
                                        }
                                    }
                                    Ok(SettingsCommand::Permissions(value)) => {
                                        match session
                                            .as_ref()
                                            .expect("session available during TUI")
                                            .update_settings(None, None, None, Some(value.clone()))
                                            .await
                                        {
                                            Ok(()) => {
                                                permissions = Some(value);
                                                app.set_settings(
                                                    model.clone(),
                                                    model_provider.clone(),
                                                    effort.clone(),
                                                    permissions.clone(),
                                                );
                                                app.projection.set_status("settings updated");
                                            }
                                            Err(error) => app.projection.set_status(error.to_string()),
                                        }
                                    }
                                    Err(error) => app.projection.set_status(error.to_string()),
                                }
                                continue;
                            }
                            let images = app.take_pending_images();
                            let remote_images = app.take_remote_image_urls();
                            let turn_input = submission_input_with_skills(
                                prompt.clone(),
                                &images,
                                &remote_images,
                                app.composer.skills(),
                            );
                            if let Some(turn_id) = app.projection.active_turn_id().map(str::to_owned) {
                                match session
                                    .as_ref()
                                    .expect("session available during TUI")
                                    .steer_turn_input(&turn_id, turn_input.clone())
                                    .await
                                {
                                    Ok(_) => {
                                        app.projection.set_status("steering");
                                        persist_prompt(
                                            session.as_ref().expect("session available during TUI"),
                                            &mut app,
                                            prompt,
                                        )
                                        .await;
                                    }
                                    Err(steer_error) => match session
                                        .as_ref()
                                        .expect("session available during TUI")
                                        .queue_input(turn_input)
                                        .await
                                    {
                                    Ok(submission) => {
                                        app.upsert_queued_submission(submission);
                                        app.projection.set_status("queued");
                                            persist_prompt(
                                                session.as_ref().expect("session available during TUI"),
                                                &mut app,
                                                prompt,
                                            )
                                            .await;
                                        }
                                        Err(queue_error) => {
                                            app.restore_pending_images(images);
                                            app.set_remote_image_urls(remote_images);
                                            app.projection.set_status(format!(
                                                "{steer_error}; queue failed: {queue_error}"
                                            ));
                                        }
                                    },
                                }
                            } else {
                                match session
                                    .as_ref()
                                    .expect("session available during TUI")
                                    .start_turn_input(turn_input)
                                    .await
                                {
                                    Ok(turn_id) => {
                                        app.start_turn(turn_id);
                                        persist_prompt(
                                            session.as_ref().expect("session available during TUI"),
                                            &mut app,
                                            prompt,
                                        )
                                        .await;
                                    }
                                    Err(error) => {
                                        app.restore_pending_images(images);
                                        app.set_remote_image_urls(remote_images);
                                        app.projection.set_status(error.to_string());
                                    }
                                }
                            }
                        }
                        AppAction::Queue(prompt) => {
                            if !app.can_accept_direct_input() {
                                continue;
                            }
                            let images = app.take_pending_images();
                            let remote_images = app.take_remote_image_urls();
                            let input = submission_input_with_skills(
                                prompt.clone(),
                                &images,
                                &remote_images,
                                app.composer.skills(),
                            );
                            match session
                                .as_ref()
                                .expect("session available during TUI")
                                .queue_input(input)
                                .await
                            {
                                Ok(submission) => {
                                    app.upsert_queued_submission(submission);
                                    app.projection.set_status("queued");
                                    persist_prompt(
                                        session.as_ref().expect("session available during TUI"),
                                        &mut app,
                                        prompt,
                                    )
                                    .await;
                                }
                                Err(error) => {
                                    app.restore_pending_images(images);
                                    app.set_remote_image_urls(remote_images);
                                    app.projection.set_status(error.to_string());
                                }
                            }
                        }
                        AppAction::EditQueuedSubmission(submission) => {
                            match session
                                .as_ref()
                                .expect("session available during TUI")
                                .delete_queued_submission(submission.id.clone())
                                .await
                            {
                                Ok(true) if app.restore_queued_submission_for_edit(submission) => {
                                    app.projection.set_status("queued input editing");
                                }
                                Ok(true) => {
                                    app.refresh_queued_submissions(
                                        session.as_ref().expect("session available during TUI"),
                                    )
                                    .await;
                                    app.projection.set_status("queued input unavailable");
                                }
                                Ok(false) => {
                                    app.refresh_queued_submissions(
                                        session.as_ref().expect("session available during TUI"),
                                    )
                                    .await;
                                    app.projection.set_status("queued input unavailable");
                                }
                                Err(error) => app
                                    .projection
                                    .set_status(format!("queue edit failed: {error}")),
                            }
                        }
                        AppAction::CopyLastResponse => {
                            copy_last_response_with(&mut app, copy_to_clipboard);
                        }
                        AppAction::ExportTranscript { path } => {
                            export_transcript_with(
                                &mut app,
                                session.as_ref().expect("session available during TUI export"),
                                path,
                                copy_to_clipboard,
                            )
                            .await;
                        }
                        AppAction::PasteImage => match paste_image_to_temp_png() {
                            Ok((path, info)) => {
                                app.attach_image(path);
                                app.projection.set_status(format!(
                                    "image attached: {}x{}",
                                    info.width, info.height
                                ));
                            }
                            Err(error) => app
                                .projection
                                .set_status(format!("image paste failed: {error}")),
                        },
                        AppAction::Interrupt => {
                            if let Some(turn_id) = app.projection.active_turn_id() {
                                let turn_id = turn_id.to_string();
                                match session
                                    .as_ref()
                                    .expect("session available during TUI")
                                    .interrupt(&turn_id)
                                    .await
                                {
                                    Ok(()) => app.projection.set_status("interrupting"),
                                    Err(error) if is_no_active_turn_error(&error) => break,
                                    Err(error) => return Err(error),
                                }
                            } else {
                                break;
                            }
                        }
                        AppAction::ScrollUp => {
                            let page_size = current_transcript_page_size(&mut terminal, &app)?;
                            app.scroll_up(page_size);
                            if app.scrollback_has_older_history {
                                if let Err(error) = app
                                    .request_older_history_page(
                                        session
                                            .as_mut()
                                            .expect("session available during TUI"),
                                    )
                                    .await
                                {
                                    app.projection
                                        .set_status(format!("history page failed: {error}"));
                                }
                            }
                        }
                        AppAction::ScrollDown => {
                            let page_size = current_transcript_page_size(&mut terminal, &app)?;
                            app.scroll_down(page_size);
                        }
                        AppAction::ScrollTop => app.scroll_top(),
                        AppAction::ScrollBottom => app.scroll_bottom(),
                        AppAction::OpenResumePicker => {
                            if app.resume_picker.is_none() {
                                let (load_tx, load_rx) = tokio::sync::mpsc::unbounded_channel();
                                let mut picker = PickerState::new(
                                    Vec::new(),
                                    SessionPickerAction::Resume,
                                    SessionStatus::Active,
                                    Some(app.cwd.clone()),
                                    false,
                                );
                                crate::resume_picker::spawn_thread_load(
                                    session
                                        .as_ref()
                                        .expect("session available during resume picker")
                                        .request_handle(),
                                    &load_tx,
                                    &mut picker,
                                );
                                app.resume_picker = Some(picker);
                                resume_picker_load_tx = Some(load_tx);
                                resume_picker_load_rx = Some(load_rx);
                            }
                        }
                        AppAction::ResumePicker(action) => {
                            let Some(picker) = app.resume_picker.as_mut() else {
                                continue;
                            };
                            let request_handle = session
                                .as_ref()
                                .expect("session available during resume picker")
                                .request_handle();
                            match action {
                                PickerAction::Select => {
                                    let selected = picker.selected_thread_id().map(ToOwned::to_owned);
                                    app.resume_picker = None;
                                    resume_picker_load_rx = None;
                                    resume_picker_load_tx = None;
                                    app.agents_overview = None;
                                    if let Some(thread_id) = selected {
                                        app.projection
                                            .set_status(format!("resuming session {thread_id}"));
                                        match app.resume_target_session(
                                            session
                                                .as_mut()
                                                .expect("session available during resume picker"),
                                            thread_id,
                                            &mut model,
                                            &mut model_provider,
                                            &mut effort,
                                            &mut permissions,
                                            &options,
                                        )
                                        .await
                                        {
                                            Ok(()) => {}
                                            Err(error) => app
                                                .projection
                                                .set_status(format!("resume failed: {error}")),
                                        }
                                    }
                                }
                                PickerAction::Restore => {
                                    if let Some(thread_id) =
                                        picker.request_unarchive_for_selected_session()
                                    {
                                        if let Some(sender) = resume_picker_load_tx.as_ref() {
                                            crate::resume_picker::spawn_unarchive_request(
                                                request_handle,
                                                sender,
                                                thread_id,
                                            );
                                        }
                                    }
                                }
                                PickerAction::Archive => {
                                    if let Some(thread_id) =
                                        picker.request_archive_for_selected_session()
                                    {
                                        if let Some(sender) = resume_picker_load_tx.as_ref() {
                                            crate::resume_picker::spawn_archive_request(
                                                request_handle,
                                                sender,
                                                thread_id,
                                            );
                                        }
                                    }
                                }
                                PickerAction::ToggleStatus
                                | PickerAction::ToggleFilter
                                | PickerAction::ToggleSort
                                | PickerAction::Reload => {
                                    match action {
                                        PickerAction::ToggleStatus => picker.toggle_status(),
                                        PickerAction::ToggleFilter => picker.toggle_filter(),
                                        PickerAction::ToggleSort => picker.toggle_sort(),
                                        PickerAction::Reload => {}
                                        _ => unreachable!(),
                                    }
                                    if let Some(sender) = resume_picker_load_tx.as_ref() {
                                        crate::resume_picker::spawn_thread_load(
                                            request_handle,
                                            sender,
                                            picker,
                                        );
                                    }
                                }
                                PickerAction::ToggleDensity => picker.toggle_density(),
                                PickerAction::ToggleExpanded => {
                                    if let Some(thread_id) = picker.toggle_selected_expansion() {
                                        if let Some(sender) = resume_picker_load_tx.as_ref() {
                                            crate::resume_picker::spawn_transcript_load(
                                                request_handle,
                                                sender,
                                                thread_id,
                                            );
                                        }
                                    }
                                }
                                PickerAction::OpenTranscript => {
                                    if let Some(thread_id) = picker.open_transcript_pager(app.locale) {
                                        if let Some(sender) = resume_picker_load_tx.as_ref() {
                                            crate::resume_picker::spawn_transcript_load(
                                                request_handle,
                                                sender,
                                                thread_id,
                                            );
                                        }
                                    }
                                }
                                PickerAction::MoveDown if picker.should_load_more() => {
                                    if let Some(sender) = resume_picker_load_tx.as_ref() {
                                        crate::resume_picker::spawn_thread_load(
                                            request_handle,
                                            sender,
                                            picker,
                                        );
                                    }
                                }
                                PickerAction::MoveUp
                                | PickerAction::MoveDown
                                | PickerAction::None
                                | PickerAction::Cancel => {}
                            }
                        }
                        AppAction::SwitchThread(thread_id) => {
                            match app.resume_target_session(
                                session.as_mut().expect("session available during TUI"),
                                thread_id,
                                &mut model,
                                &mut model_provider,
                                &mut effort,
                                &mut permissions,
                                &options,
                            )
                            .await
                            {
                                Ok(()) => app.projection.set_status("switched agent"),
                                Err(error) => app.projection.set_status(format!(
                                    "agent switch failed: {error}"
                                )),
                            }
                        }
                        AppAction::DecreaseEffort
                        | AppAction::IncreaseEffort
                        | AppAction::PreviousPermissions
                        | AppAction::NextPermissions
                        | AppAction::Respond(_)
                        | AppAction::SelectModel(_)
                        | AppAction::ChangeCollaborationMode(_)
                        | AppAction::RefreshAgentsOverview
                        | AppAction::DispatchAgentsOverviewTask { .. }
                        | AppAction::RenameAgentsOverviewThread { .. }
                        | AppAction::StopAgentsOverviewThread { .. } => {
                            unreachable!("App Server actions are handled by app::event_dispatch")
                        }
                        AppAction::Quit => break,
                        AppAction::None => {}
                    }
                }
                event = async {
                    match session.as_mut() {
                        Some(session) => session.next_event().await,
                        None => std::future::pending::<Option<AppServerEvent>>().await,
                    }
                }, if session.is_some() => {
                    let event = event.unwrap_or_else(|| AppServerEvent::Disconnected {
                        message: "app-server event stream closed".to_string(),
                    });
                    let disconnected_message = match &event {
                        AppServerEvent::Disconnected { message } => Some(message.clone()),
                        _ => None,
                    };
                    app.handle_app_server_event(
                        session.as_ref().expect("session available during TUI"),
                        event,
                    )
                    .await;
                    if disconnected_message.is_some() {
                        let thread_id = session
                                .as_ref()
                                .expect("session available during TUI")
                                .thread_id()?
                            .to_string();
                            terminal
                                .terminal_mut()
                                .draw(|frame| view::render(frame, &app))
                                .context("failed to render reconnecting state")?;
                            let old_session = session
                                .take()
                                .expect("session available during reconnect");
                            let _ = old_session.shutdown().await;
                            reconnect_thread_id = Some(thread_id);
                            reconnect_failed = false;
                    }
                }
                result = async {
                    match reconnect.as_mut() {
                        Some(future) => future.await,
                        None => std::future::pending::<Result<ReconnectedSession>>().await,
                    }
                }, if reconnect.is_some() => {
                    reconnect = None;
                    match result {
                        Ok(reconnected) => {
                            let active_profile = reconnected
                                .session
                                .active_permission_profile()
                                .map(str::to_string);
                            app.hydrate_thread(reconnected.thread);
                            crate::app::working_directory::sync_server_cwd(
                                &mut app,
                                reconnected.cwd,
                            );
                            if let Some(history_page) = reconnected.history_page {
                                app.prepend_initial_history_page(history_page);
                            }
                            app.scrollback_has_older_history =
                                reconnected.scrollback_has_older_history;
                            app.set_permission_profiles(reconnected.permission_profiles);
                            app.set_collaboration_modes(
                                reconnected
                                    .session
                                    .list_collaboration_modes()
                                    .await
                                    .unwrap_or_default(),
                            );
                            if options.permissions.is_none() {
                                if let Some(active_profile) = active_profile {
                                    permissions = Some(active_profile);
                                }
                            }
                            app.set_settings(
                                model.clone(),
                                model_provider.clone(),
                                effort.clone(),
                                permissions.clone(),
                            );
                            app.refresh_queued_submissions(&reconnected.session).await;
                            session = Some(reconnected.session);
                            reconnect_thread_id = None;
                            reconnect_failed = false;
                            app.projection.set_status("reconnected");
                        }
                        Err(error) => {
                            reconnect_failed = true;
                            app.projection.set_status(format!("reconnect failed: {error}"));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    app.end_startup_input_boundary();
    let restore_result = terminal.restore().context("failed to restore terminal");
    let shutdown_result = match session {
        Some(session) => session.shutdown().await,
        None => Ok(()),
    };
    run_result?;
    restore_result?;
    shutdown_result
}

fn is_no_active_turn_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        let message = cause.to_string().to_ascii_lowercase();
        message.contains("no active turn") || message.contains("turn_not_active")
    })
}

fn current_transcript_page_size(terminal: &mut Tui, app: &App) -> Result<usize> {
    let size = terminal
        .terminal_mut()
        .size()
        .context("failed to read terminal size")?;
    Ok(view::transcript_page_size(size.width, size.height, app))
}

/// Resume without an explicit id using the same canonical thread picker as Codex.
/// The picker session is short-lived; the selected thread is then resumed by the
/// normal TUI runtime so there is only one conversation owner.
pub async fn run_resume(mut options: TuiOptions) -> Result<()> {
    if options.resume_thread.is_none() {
        let selected = run_resume_picker_with_app_server(&options).await?;
        let Some(thread_id) = selected else {
            return Ok(());
        };
        options.resume_thread = Some(thread_id);
    }
    run_tui(options).await
}

async fn persist_prompt(session: &AppServerSession, app: &mut App, prompt: String) {
    if prompt.trim().is_empty() {
        return;
    }
    if let Err(error) = session.append_prompt_history(prompt).await {
        app.projection
            .set_status(format!("prompt history unavailable: {error}"));
    }
}

#[cfg(test)]
fn submission_input(
    prompt: String,
    images: &[PathBuf],
    remote_images: &[String],
) -> Vec<UserInput> {
    submission_input_with_skills(prompt, images, remote_images, &[])
}

fn submission_input_with_skills(
    prompt: String,
    images: &[PathBuf],
    remote_images: &[String],
    skills: &[app_server_protocol::protocol::v2::SkillMetadata],
) -> Vec<UserInput> {
    let mut input = images
        .iter()
        .map(|path| UserInput::LocalImage {
            detail: None,
            path: path.to_string_lossy().into_owned(),
        })
        .collect::<Vec<_>>();
    input.extend(remote_images.iter().map(|url| UserInput::Image {
        detail: None,
        url: url.clone(),
    }));
    let mut seen = std::collections::HashSet::new();
    for token in prompt.split_whitespace() {
        let Some(name) = token.strip_prefix('$') else {
            continue;
        };
        let Some(skill) = skills.iter().find(|skill| skill.name == name) else {
            continue;
        };
        if seen.insert(skill.name.clone()) {
            input.push(UserInput::Skill {
                name: skill.name.clone(),
                path: skill.path.to_string_lossy().into_owned(),
            });
        }
    }
    if !prompt.is_empty() {
        input.push(UserInput::Text {
            text: prompt,
            text_elements: Vec::new(),
        });
    }
    input
}

fn copy_last_response_with(
    app: &mut App,
    copy: impl FnOnce(&str) -> Result<Option<crate::clipboard_copy::ClipboardLease>, String>,
) {
    let response = app.projection.final_answer();
    if response.is_empty() {
        app.projection.set_status("no agent response to copy");
        return;
    }
    match copy(&response) {
        Ok(lease) => {
            app.clipboard_lease = lease;
            app.projection.set_status("copied last response");
        }
        Err(error) => app.projection.set_status(format!("copy failed: {error}")),
    }
}

async fn export_transcript_with(
    app: &mut App,
    session: &AppServerSession,
    path: Option<std::path::PathBuf>,
    copy: impl FnOnce(&str) -> Result<Option<crate::clipboard_copy::ClipboardLease>, String>,
) {
    let live_entries = app.projection.entries();
    let entries = match session.thread_id() {
        Ok(thread_id) => match crate::thread_transcript::load_session_transcript_with_handle(
            session.request_handle(),
            thread_id.to_string(),
        )
        .await
        {
            Ok(entries) => merge_export_entries(entries, live_entries),
            Err(_) => live_entries.to_vec(),
        },
        Err(_) => live_entries.to_vec(),
    };
    let markdown = match crate::app::transcript_export::render_markdown_transcript(&entries) {
        Ok(markdown) => markdown,
        Err(error) => {
            app.projection.set_status(format!("export failed: {error}"));
            return;
        }
    };
    match path {
        Some(path) => {
            match crate::app::transcript_export::write_transcript(&app.cwd, &path, &markdown) {
                Ok(path) => app
                    .projection
                    .set_status(format!("exported conversation to {}", path.display())),
                Err(error) => app.projection.set_status(format!("export failed: {error}")),
            }
        }
        None => match copy(&markdown) {
            Ok(lease) => {
                app.clipboard_lease = lease;
                app.projection
                    .set_status("exported conversation to clipboard");
            }
            Err(error) => app.projection.set_status(format!("export failed: {error}")),
        },
    }
}

fn merge_export_entries(
    mut persisted: Vec<TranscriptEntry>,
    live: &[TranscriptEntry],
) -> Vec<TranscriptEntry> {
    for live_entry in live {
        if let Some(persisted_entry) = persisted
            .iter_mut()
            .find(|persisted_entry| persisted_entry.id == live_entry.id)
        {
            *persisted_entry = live_entry.clone();
        } else {
            persisted.push(live_entry.clone());
        }
    }
    persisted
}

pub async fn run_exec(options: ExecOptions) -> Result<ExecResult> {
    validate_model_route(&options.tui)?;
    if let Some(remote) = options.tui.remote.clone() {
        let session = AppServerSession::connect_remote(remote).await?;
        run_exec_with_session(options, session).await
    } else {
        let config = stdio_config(&options.tui)?;
        run_exec_with_config(options, config).await
    }
}

pub(crate) async fn connect_session(options: &TuiOptions) -> Result<AppServerSession> {
    if let Some(remote) = options.remote.clone() {
        AppServerSession::connect_remote(remote).await
    } else {
        AppServerSession::connect(stdio_config(options)?).await
    }
}

pub(crate) fn stdio_config(options: &TuiOptions) -> Result<StdioTransportConfig> {
    validate_model_route(options)?;
    let mut config = StdioTransportConfig::runtime(&options.app_server_bin);
    config.args.extend(options.app_server_args.iter().cloned());
    Ok(config)
}

async fn run_exec_with_config(
    options: ExecOptions,
    config: StdioTransportConfig,
) -> Result<ExecResult> {
    let session = AppServerSession::connect(config).await?;
    run_exec_with_session(options, session).await
}

async fn run_exec_with_session(
    options: ExecOptions,
    mut session: AppServerSession,
) -> Result<ExecResult> {
    if options.prompt.trim().is_empty() {
        bail!("prompt must not be empty");
    }
    let execution = async {
        let thread_id = if let Some(thread_id) = options.tui.resume_thread.clone() {
            let response = session.resume_thread(thread_id).await?;
            let thread_id = response.thread.id.clone();
            let mut projection = ConversationProjection::default();
            projection.hydrate_thread(response.thread);
            (thread_id, projection)
        } else {
            let response = session
                .start_thread(
                    options.tui.cwd.clone(),
                    options.tui.model.clone(),
                    options.tui.model_provider.clone(),
                )
                .await?;
            (
                response.thread.id.clone(),
                ConversationProjection::default(),
            )
        };
        session
            .update_settings(
                options.tui.model.clone(),
                options.tui.model_provider.clone(),
                options.tui.reasoning_effort.clone(),
                options.tui.permissions.clone(),
            )
            .await?;
        let (thread_id, mut projection) = thread_id;
        let turn_id = session.start_turn(options.prompt).await?;

        loop {
            let event = session
                .next_event()
                .await
                .ok_or_else(|| anyhow!("App Server disconnected before turn completion"))?;
            match event {
                AppServerEvent::ServerNotification(notification) => {
                    let completed = matches!(
                        notification.as_ref(),
                        ServerNotification::TurnCompleted(params) if params.turn.id == turn_id
                    );
                    projection.apply(*notification);
                    if completed {
                        break;
                    }
                }
                AppServerEvent::ServerRequest(request) => match *request {
                    ServerRequest::CurrentTimeRead { id, .. } => {
                        session.respond_current_time(id).await?;
                    }
                    request => match AppServerResponse::fail_closed(request) {
                        Ok(response) => session.respond(response).await?,
                        Err(request) => session.reject_server_request(request).await?,
                    },
                },
                AppServerEvent::Disconnected { message } => bail!(message),
                AppServerEvent::Lagged { .. } => {}
            }
        }

        Ok(ExecResult {
            thread_id,
            turn_id,
            status: projection.status().to_string(),
            output: projection.final_answer(),
        })
    };
    let execution = execution.await;
    let shutdown = session.shutdown().await;
    match execution {
        Ok(result) => {
            shutdown?;
            Ok(result)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "runtime_pty_tests.rs"]
mod pty_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{EntryKind, EntryStatus};
    use app_server_protocol::protocol::v2::AgentMessageDeltaNotification;
    use std::cell::RefCell;
    use std::ffi::OsString;

    fn transcript_entry(id: &str, text: &str, status: Option<EntryStatus>) -> TranscriptEntry {
        TranscriptEntry {
            id: id.to_string(),
            kind: EntryKind::Assistant,
            text: text.to_string(),
            streaming: status == Some(EntryStatus::Running),
            status,
            summary: Vec::new(),
        }
    }

    #[test]
    fn submission_input_keeps_codex_image_then_text_order() {
        let input = submission_input(
            "describe these".to_string(),
            &[PathBuf::from("one.png"), PathBuf::from("two.png")],
            &["https://example.test/remote.png".to_string()],
        );

        assert_eq!(
            input,
            vec![
                UserInput::LocalImage {
                    detail: None,
                    path: "one.png".to_string(),
                },
                UserInput::LocalImage {
                    detail: None,
                    path: "two.png".to_string(),
                },
                UserInput::Image {
                    detail: None,
                    url: "https://example.test/remote.png".to_string(),
                },
                UserInput::Text {
                    text: "describe these".to_string(),
                    text_elements: Vec::new(),
                },
            ]
        );
        assert_eq!(
            submission_input(String::new(), &[PathBuf::from("only.png")], &[]),
            vec![UserInput::LocalImage {
                detail: None,
                path: "only.png".to_string(),
            }]
        );
    }

    #[test]
    fn submission_input_keeps_remote_images_before_text() {
        assert_eq!(
            submission_input(
                "describe".to_string(),
                &[],
                &["https://example.test/one.png".to_string()],
            ),
            vec![
                UserInput::Image {
                    detail: None,
                    url: "https://example.test/one.png".to_string(),
                },
                UserInput::Text {
                    text: "describe".to_string(),
                    text_elements: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn submission_input_adds_loaded_skills_without_dropping_prompt_text() {
        let skill = app_server_protocol::protocol::v2::SkillMetadata {
            name: "review".to_string(),
            description: "Review code".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            path: PathBuf::from("/skills/review/SKILL.md"),
            scope: app_server_protocol::protocol::v2::SkillScope::User,
            enabled: true,
        };
        assert_eq!(
            submission_input_with_skills("please use $review".to_string(), &[], &[], &[skill],),
            vec![
                UserInput::Skill {
                    name: "review".to_string(),
                    path: "/skills/review/SKILL.md".to_string(),
                },
                UserInput::Text {
                    text: "please use $review".to_string(),
                    text_elements: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn export_merge_keeps_persisted_order_and_latest_live_entries() {
        let persisted = vec![
            transcript_entry("history", "older", Some(EntryStatus::Completed)),
            transcript_entry("shared", "persisted", Some(EntryStatus::Completed)),
        ];
        let live = vec![
            transcript_entry("shared", "streaming", Some(EntryStatus::Running)),
            transcript_entry("live", "new", Some(EntryStatus::Running)),
        ];

        let merged = merge_export_entries(persisted, &live);

        assert_eq!(
            merged
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history", "shared", "live"]
        );
        assert_eq!(merged[1].text, "streaming");
        assert_eq!(merged[1].status, Some(EntryStatus::Running));
        assert_eq!(merged[2], live[1]);
    }

    #[tokio::test]
    async fn real_stdio_unavailable_backend_fails_closed_without_provider() {
        let Some(app_server_bin) = std::env::var_os("LIME_TEST_APP_SERVER_BIN") else {
            return;
        };
        let temp_dir = tempfile::tempdir().expect("temp data directory");
        let tui = TuiOptions {
            app_server_bin: PathBuf::from(&app_server_bin),
            app_server_args: Vec::new(),
            remote: None,
            cwd: temp_dir.path().to_path_buf(),
            model: None,
            model_provider: None,
            reasoning_effort: None,
            permissions: None,
            locale: None,
            resume_thread: None,
        };
        let config = StdioTransportConfig {
            app_server_bin: PathBuf::from(app_server_bin),
            args: vec![
                OsString::from("--stdio"),
                OsString::from("--backend"),
                OsString::from("unavailable"),
                OsString::from("--data-dir"),
                temp_dir.path().as_os_str().to_os_string(),
            ],
        };

        let error = run_exec_with_config(
            ExecOptions {
                tui,
                prompt: "stdio contract probe".to_string(),
            },
            config,
        )
        .await
        .expect_err("unavailable backend must reject turn/start");

        let rendered = format!("{error:#}");
        assert!(
            rendered.contains("failed to start App Server thread")
                && rendered.contains("runtime model route is not executable"),
            "unexpected fail-closed error: {rendered}"
        );
    }

    #[test]
    fn model_route_requires_model_and_provider_together() {
        let options = TuiOptions {
            app_server_bin: PathBuf::from("app-server"),
            app_server_args: Vec::new(),
            remote: None,
            cwd: PathBuf::from("."),
            model: Some("gpt-test".to_string()),
            model_provider: None,
            reasoning_effort: None,
            permissions: None,
            locale: None,
            resume_thread: None,
        };

        assert_eq!(
            validate_model_route(&options)
                .expect_err("partial route must fail")
                .to_string(),
            "--model and --provider must be specified together"
        );
    }

    #[test]
    fn stdio_config_appends_host_arguments_after_the_current_runtime_default() {
        let options = TuiOptions {
            app_server_bin: PathBuf::from("custom-app-server"),
            app_server_args: vec![OsString::from("--backend"), OsString::from("external")],
            remote: None,
            cwd: PathBuf::from("."),
            model: Some("fixture-model".to_string()),
            model_provider: Some("fixture-provider".to_string()),
            reasoning_effort: None,
            permissions: None,
            locale: None,
            resume_thread: None,
        };

        let config = stdio_config(&options).expect("stdio config");
        assert_eq!(
            config.args,
            vec![
                OsString::from("--stdio"),
                OsString::from("--backend"),
                OsString::from("runtime"),
                OsString::from("--backend"),
                OsString::from("external"),
            ]
        );
    }

    #[test]
    fn copy_uses_last_canonical_agent_markdown_and_reports_outcome() {
        let mut app = App::default();
        app.projection.apply(ServerNotification::AgentMessageDelta(
            AgentMessageDeltaNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "message-1".to_string(),
                delta: "**answer** with `code`".to_string(),
            },
        ));
        let copied = RefCell::new(String::new());

        copy_last_response_with(&mut app, |text| {
            copied.replace(text.to_string());
            Ok(Some(crate::clipboard_copy::ClipboardLease::test()))
        });

        assert_eq!(copied.into_inner(), "**answer** with `code`");
        assert_eq!(app.projection.status(), "copied last response");
        assert!(app.clipboard_lease.is_some());

        let mut empty = App::default();
        copy_last_response_with(&mut empty, |_| panic!("clipboard must not be called"));
        assert_eq!(empty.projection.status(), "no agent response to copy");
    }
}
