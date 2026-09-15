//! Semantic styles shared by Lime TUI surfaces.

use ratatui::style::{Color, Style};

use crate::terminal_palette::{
    best_color_for_level, default_bg, default_fg, effective_stdout_color_level, rgb_color,
    stdout_color_level, StdoutColorLevel,
};

const LIGHT_BG_ACCENT_RGB: (u8, u8, u8) = (0, 95, 135);
const TABLE_SEPARATOR_FG_ALPHA: f32 = 0.20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusTone {
    Success,
    Attention,
    Failure,
}

pub(crate) fn status_style(tone: StatusTone) -> Style {
    status_style_for(tone, default_bg(), effective_stdout_color_level())
}

pub(crate) fn attention_style() -> Style {
    status_style(StatusTone::Attention)
}

pub(crate) fn failure_style() -> Style {
    status_style(StatusTone::Failure)
}

pub(crate) fn accent_style() -> Style {
    accent_style_for(default_bg(), effective_stdout_color_level())
}

pub(crate) fn muted_style() -> Style {
    Style::default().dim()
}

pub(crate) fn user_message_style() -> Style {
    user_message_style_for(default_bg(), effective_stdout_color_level())
}

pub(crate) fn table_separator_style() -> Style {
    table_separator_style_for(default_fg(), default_bg(), stdout_color_level())
}

pub(crate) fn footer_hint_label_style() -> Style {
    footer_hint_label_style_for(default_bg(), effective_stdout_color_level())
}

fn status_style_for(
    tone: StatusTone,
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    let base = Style::default().bold();
    if color_level == StdoutColorLevel::Unknown {
        return base;
    }

    let light = terminal_bg.is_some_and(is_light);
    match tone {
        StatusTone::Success => base.fg(Color::Green),
        StatusTone::Failure => base.fg(Color::Red),
        StatusTone::Attention if light || terminal_bg.is_none() => base,
        StatusTone::Attention => base.fg(Color::Yellow),
    }
}

fn accent_style_for(terminal_bg: Option<(u8, u8, u8)>, color_level: StdoutColorLevel) -> Style {
    let base = Style::default().bold();
    if color_level == StdoutColorLevel::Unknown {
        return base;
    }
    if terminal_bg.is_some_and(is_light) {
        base.fg(best_color_for_level(LIGHT_BG_ACCENT_RGB, color_level))
    } else {
        base.fg(Color::Cyan)
    }
}

fn user_message_style_for(
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    let Some(background) = terminal_bg else {
        return Style::default();
    };
    if matches!(
        color_level,
        StdoutColorLevel::Ansi16 | StdoutColorLevel::Unknown
    ) {
        return Style::default();
    }
    Style::default().bg(best_color_for_level(
        user_message_bg_rgb(background),
        color_level,
    ))
}

fn table_separator_style_for(
    terminal_fg: Option<(u8, u8, u8)>,
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    let (Some(foreground), Some(background)) = (terminal_fg, terminal_bg) else {
        return muted_style();
    };
    let separator = blend(foreground, background, TABLE_SEPARATOR_FG_ALPHA);
    match color_level {
        StdoutColorLevel::TrueColor => Style::default().fg(rgb_color(separator)),
        StdoutColorLevel::Ansi256 => {
            Style::default().fg(best_color_for_level(separator, color_level))
        }
        StdoutColorLevel::Ansi16 | StdoutColorLevel::Unknown => muted_style(),
    }
}

fn footer_hint_label_style_for(
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    if color_level != StdoutColorLevel::Unknown && terminal_bg.is_some_and(is_light) {
        Style::default().fg(Color::DarkGray)
    } else {
        muted_style()
    }
}

fn user_message_bg_rgb(background: (u8, u8, u8)) -> (u8, u8, u8) {
    let (foreground, alpha) = if is_light(background) {
        ((0, 0, 0), 0.04)
    } else {
        ((255, 255, 255), 0.12)
    };
    blend(foreground, background, alpha)
}

fn is_light((red, green, blue): (u8, u8, u8)) -> bool {
    let luminance = 0.299 * f32::from(red) + 0.587 * f32::from(green) + 0.114 * f32::from(blue);
    luminance > 128.0
}

fn blend(foreground: (u8, u8, u8), background: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
    let channel = |foreground: u8, background: u8| {
        (f32::from(foreground) * alpha + f32::from(background) * (1.0 - alpha)) as u8
    };
    (
        channel(foreground.0, background.0),
        channel(foreground.1, background.1),
        channel(foreground.2, background.2),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn status_tones_preserve_light_dark_and_no_color_semantics() {
        assert_eq!(
            status_style_for(
                StatusTone::Attention,
                Some((0, 0, 0)),
                StdoutColorLevel::Ansi16,
            )
            .fg,
            Some(Color::Yellow),
        );
        assert_eq!(
            status_style_for(
                StatusTone::Attention,
                Some((255, 255, 255)),
                StdoutColorLevel::TrueColor,
            )
            .fg,
            None,
        );
        for tone in [
            StatusTone::Success,
            StatusTone::Attention,
            StatusTone::Failure,
        ] {
            let style = status_style_for(tone, Some((0, 0, 0)), StdoutColorLevel::Unknown);
            assert_eq!(style.fg, None);
            assert!(style.add_modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn accent_is_palette_aware_and_has_a_no_color_fallback() {
        assert_eq!(
            accent_style_for(Some((0, 0, 0)), StdoutColorLevel::Ansi16).fg,
            Some(Color::Cyan),
        );
        assert!(matches!(
            accent_style_for(Some((255, 255, 255)), StdoutColorLevel::TrueColor).fg,
            Some(Color::Rgb(0, 95, 135)),
        ));
        assert_eq!(
            accent_style_for(Some((0, 0, 0)), StdoutColorLevel::Unknown).fg,
            None,
        );
    }

    #[test]
    fn user_message_surface_adapts_to_dark_light_and_no_color() {
        assert_eq!(
            user_message_style_for(Some((0, 0, 0)), StdoutColorLevel::TrueColor).bg,
            Some(Color::Rgb(30, 30, 30)),
        );
        assert_eq!(
            user_message_style_for(Some((255, 255, 255)), StdoutColorLevel::TrueColor).bg,
            Some(Color::Rgb(244, 244, 244)),
        );
        assert_eq!(
            user_message_style_for(Some((0, 0, 0)), StdoutColorLevel::Unknown).bg,
            None,
        );
    }

    #[test]
    fn table_separator_blends_on_truecolor_and_dims_without_palette_support() {
        assert_eq!(
            table_separator_style_for(
                Some((255, 255, 255)),
                Some((0, 0, 0)),
                StdoutColorLevel::TrueColor,
            )
            .fg,
            Some(Color::Rgb(51, 51, 51)),
        );
        assert!(table_separator_style_for(
            Some((255, 255, 255)),
            Some((0, 0, 0)),
            StdoutColorLevel::Unknown,
        )
        .add_modifier
        .contains(Modifier::DIM),);
    }

    #[test]
    fn footer_styles_keep_labels_quiet_on_all_palettes() {
        assert_eq!(
            footer_hint_label_style_for(Some((255, 255, 255)), StdoutColorLevel::TrueColor,).fg,
            Some(Color::DarkGray),
        );
        assert!(
            footer_hint_label_style_for(Some((0, 0, 0)), StdoutColorLevel::Unknown)
                .add_modifier
                .contains(Modifier::DIM),
        );
    }
}
