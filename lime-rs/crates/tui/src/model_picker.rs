use app_server_protocol::protocol::v2::Model;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::Frame;
use std::cell::Cell;
use unicode_segmentation::UnicodeSegmentation;

use crate::bottom_pane::selection_row_layout::MAX_POPUP_ROWS;
use crate::locale::Locale;

mod render;
pub(crate) use render::desired_height;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelSelection {
    pub(crate) model: String,
    pub(crate) provider: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelPickerAction {
    None,
    Cancel,
    Select(usize),
}

#[derive(Debug, Default)]
pub(crate) struct ModelPicker {
    models: Vec<Model>,
    selected: usize,
    query: String,
    current: Option<usize>,
    page_rows: Cell<usize>,
}

impl ModelPicker {
    pub(crate) fn new(models: Vec<Model>) -> Self {
        let mut models = models
            .into_iter()
            .filter(|model| !model.hidden)
            .collect::<Vec<_>>();
        models.sort_by_key(|model| (!model.is_default, model.display_name.to_lowercase()));
        Self {
            models,
            selected: 0,
            query: String::new(),
            current: None,
            page_rows: Cell::new(MAX_POPUP_ROWS),
        }
    }

    pub(crate) fn with_current(mut self, model: Option<&str>, provider: Option<&str>) -> Self {
        let mut matches = self
            .models
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                Some(entry.model.as_str()) == model
                    && provider.is_none_or(|provider| entry.provider_id == provider)
            })
            .map(|(index, _)| index);
        let first = matches.next();
        // Without provider identity a shared model name is ambiguous; never guess the first one.
        self.current = first.filter(|_| matches.next().is_none());
        if let Some(index) = self.current {
            self.selected = index;
        }
        self
    }

    pub(crate) fn selected_model(&self, index: usize) -> Option<ModelSelection> {
        self.visible_indices()
            .get(index)
            .and_then(|model_index| self.models.get(*model_index))
            .map(|model| ModelSelection {
                model: model.model.clone(),
                provider: model.provider_id.clone(),
            })
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    #[cfg(test)]
    fn visible_models(&self) -> Vec<&Model> {
        self.visible_indices()
            .into_iter()
            .filter_map(|index| self.models.get(index))
            .collect()
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> ModelPickerAction {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
                {
                    return ModelPickerAction::Cancel;
                }
                match key.code {
                    KeyCode::Esc => ModelPickerAction::Cancel,
                    KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k')
                        if key.code == KeyCode::Up
                            || key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        let count = self.visible_indices().len();
                        if count > 0 {
                            self.selected = self
                                .selected
                                .checked_sub(1)
                                .unwrap_or(count.saturating_sub(1));
                        }
                        ModelPickerAction::None
                    }
                    KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j')
                        if key.code == KeyCode::Down
                            || key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        let count = self.visible_indices().len();
                        if count > 0 {
                            self.selected = (self.selected + 1) % count;
                        }
                        ModelPickerAction::None
                    }
                    KeyCode::Enter => self
                        .selected_model(self.selected)
                        .map(|_| ModelPickerAction::Select(self.selected))
                        .unwrap_or(ModelPickerAction::None),
                    KeyCode::PageUp => {
                        self.selected = self.selected.saturating_sub(self.page_rows.get().max(1));
                        ModelPickerAction::None
                    }
                    KeyCode::PageDown => {
                        self.selected = self
                            .selected
                            .saturating_add(self.page_rows.get().max(1))
                            .min(self.visible_indices().len().saturating_sub(1));
                        ModelPickerAction::None
                    }
                    KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.selected = 0;
                        ModelPickerAction::None
                    }
                    KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.selected = self.visible_indices().len().saturating_sub(1);
                        ModelPickerAction::None
                    }
                    KeyCode::Backspace => {
                        if let Some((offset, _)) = self.query.grapheme_indices(true).next_back() {
                            self.query.truncate(offset);
                        }
                        self.selected = 0;
                        ModelPickerAction::None
                    }
                    KeyCode::Char(ch)
                        if !ch.is_control()
                            && !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        self.query.push(ch);
                        self.selected = 0;
                        ModelPickerAction::None
                    }
                    _ => ModelPickerAction::None,
                }
            }
            Event::Paste(text) => {
                if let Some(text) = crate::clipboard_paste::normalize_pasted_search_query(&text) {
                    self.query.push_str(&text);
                    self.selected = 0;
                }
                ModelPickerAction::None
            }
            _ => ModelPickerAction::None,
        }
    }

    fn visible_indices(&self) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        self.models
            .iter()
            .enumerate()
            .filter(|(_, model)| {
                query.is_empty()
                    || model.display_name.to_lowercase().contains(&query)
                    || model.model.to_lowercase().contains(&query)
                    || model.provider_id.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
pub(crate) fn render(frame: &mut Frame<'_>, area: Rect, picker: &ModelPicker) {
    render_with_locale(frame, area, picker, Locale::default());
}

pub(crate) fn render_with_locale(
    frame: &mut Frame<'_>,
    area: Rect,
    picker: &ModelPicker,
    locale: Locale,
) {
    render::render(frame, area, picker, locale);
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{InputModality, Model};
    use app_server_protocol::CapabilitySnapshot;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    pub(super) fn model(id: &str, provider: &str, hidden: bool, is_default: bool) -> Model {
        Model {
            id: id.to_string(),
            provider_id: provider.to_string(),
            model: id.to_string(),
            upgrade: None,
            upgrade_info: None,
            availability_nux: None,
            display_name: id.to_string(),
            description: String::new(),
            hidden,
            supported_reasoning_efforts: Vec::new(),
            default_reasoning_effort: "medium".to_string(),
            input_modalities: vec![InputModality::Text],
            capability_snapshot: CapabilitySnapshot::default(),
            context_window: None,
            max_output_tokens: None,
            supports_personality: false,
            multi_agent_version: None,
            additional_speed_tiers: Vec::new(),
            service_tiers: Vec::new(),
            default_service_tier: None,
            is_default,
        }
    }

    #[test]
    fn picker_filters_hidden_models_and_selects_default_first() {
        let mut picker = ModelPicker::new(vec![
            model("slow", "fixture", false, false),
            model("hidden", "fixture", true, false),
            model("fast", "fixture", false, true),
        ]);
        assert_eq!(picker.visible_models().len(), 2);
        assert_eq!(picker.selected_model(0).expect("default").model, "fast");
        picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('l'),
            KeyModifiers::NONE,
        )));
        assert_eq!(picker.selected_model(0).expect("filtered").model, "slow");
        assert_eq!(
            picker.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            ModelPickerAction::Select(0)
        );
    }

    #[test]
    fn searchable_picker_keeps_plain_vim_letters_for_query_input() {
        let mut picker = ModelPicker::new(vec![
            model("alpha", "fixture", false, true),
            model("jupiter", "fixture", false, false),
            model("kilo", "fixture", false, false),
        ]);

        assert_eq!(
            picker.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
            ))),
            ModelPickerAction::None
        );
        assert_eq!(picker.query(), "j");
        assert_eq!(picker.visible_models()[0].model, "jupiter");

        picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::NONE,
        )));
        picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::NONE,
        )));
        assert_eq!(picker.query(), "k");
        assert_eq!(picker.visible_models()[0].model, "kilo");
    }

    #[test]
    fn picker_navigation_wraps_and_accepts_control_bindings() {
        let mut picker = ModelPicker::new(vec![
            model("first", "fixture", false, true),
            model("second", "fixture", false, false),
        ]);

        picker.handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert_eq!(picker.selected, 1);

        picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(picker.selected, 0);

        picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(picker.selected, 1);
    }

    #[test]
    fn unhandled_control_keys_do_not_pollute_model_query() {
        let mut picker = ModelPicker::new(vec![model("first", "fixture", false, true)]);
        picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL,
        )));
        assert!(picker.query().is_empty());
    }

    #[test]
    fn picker_escape_cancels_and_render_stays_bounded() {
        let mut picker = ModelPicker::new(vec![model("模型", "提供方", false, true)]);
        assert_eq!(
            picker.handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))),
            ModelPickerAction::Cancel
        );
        let mut terminal = Terminal::new(TestBackend::new(12, 6)).expect("terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), &picker))
            .expect("draw");
    }

    #[test]
    fn long_model_catalog_keeps_selected_row_inside_bounded_popup() {
        let models = (0..20)
            .map(|index| model(&format!("model-{index:02}"), "fixture", false, index == 0))
            .collect();
        let mut picker = ModelPicker::new(models);
        for _ in 0..12 {
            picker.handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
        }

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), &picker))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let text = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("model-12"),
            "selected model was clipped: {text}"
        );
        assert!(
            text.contains("› 13. model-12"),
            "selected model marker missing: {text}"
        );
        assert!(text.lines().all(|line| line.chars().count() <= 80));
    }
}
