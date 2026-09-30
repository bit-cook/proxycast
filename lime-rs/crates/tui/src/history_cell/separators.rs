//! Turn separators for transcript history.

use super::*;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy)]
pub struct FinalMessageSeparator {
    elapsed_seconds: Option<u64>,
    locale: Locale,
}

impl FinalMessageSeparator {
    pub(crate) fn new(elapsed_seconds: Option<u64>) -> Self {
        Self {
            elapsed_seconds,
            locale: Locale::default(),
        }
    }

    pub(crate) fn with_locale(mut self, locale: Locale) -> Self {
        self.locale = locale;
        self
    }

    fn worked_label(&self, seconds: u64) -> String {
        let elapsed = if seconds == 0 {
            match self.locale {
                Locale::ZhCn => "小于 1 秒",
                Locale::ZhTw => "少於 1 秒",
                Locale::EnUs => "<1s",
                Locale::JaJp => "1秒未満",
                Locale::KoKr => "1초 미만",
            }
            .to_string()
        } else {
            format!(
                "{seconds}{}",
                match self.locale {
                    Locale::ZhCn => " 秒",
                    Locale::ZhTw => " 秒",
                    Locale::EnUs => "s",
                    Locale::JaJp => "秒",
                    Locale::KoKr => "초",
                }
            )
        };
        match self.locale {
            Locale::ZhCn => format!("已用时 {elapsed}"),
            Locale::ZhTw => format!("已用時 {elapsed}"),
            Locale::EnUs => format!("Worked for {elapsed}"),
            Locale::JaJp => format!("所要時間 {elapsed}"),
            Locale::KoKr => format!("소요 시간 {elapsed}"),
        }
    }
}

impl HistoryCell for FinalMessageSeparator {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let label = self
            .elapsed_seconds
            .map(|seconds| format!(" {} ", self.worked_label(seconds)));
        let label_width = label
            .as_deref()
            .map(UnicodeWidthStr::width)
            .unwrap_or_default();
        let line = match label {
            Some(label) if label_width < usize::from(width) => format!(
                "-{label}{}",
                "-".repeat(usize::from(width).saturating_sub(label_width + 1))
            ),
            _ => "-".repeat(usize::from(width)),
        };
        vec![Line::from(line)]
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.elapsed_seconds
            .map(|seconds| vec![Line::from(self.worked_label(seconds))])
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_separator_localizes_elapsed_label() {
        let labels = [
            (Locale::ZhCn, "已用时 61 秒"),
            (Locale::ZhTw, "已用時 61 秒"),
            (Locale::EnUs, "Worked for 61s"),
            (Locale::JaJp, "所要時間 61秒"),
            (Locale::KoKr, "소요 시간 61초"),
        ];
        for (locale, expected) in labels {
            let lines = FinalMessageSeparator::new(Some(61))
                .with_locale(locale)
                .raw_lines();
            assert_eq!(lines[0].to_string(), expected);

            let display_lines = FinalMessageSeparator::new(Some(61))
                .with_locale(locale)
                .display_lines(20);
            assert!(display_lines.iter().all(|line| line.width() <= 20));
        }
    }

    #[test]
    fn completion_separator_keeps_short_elapsed_labels_visible() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let lines = FinalMessageSeparator::new(Some(12))
                .with_locale(locale)
                .raw_lines();
            assert_eq!(lines.len(), 1);
            assert!(lines[0].to_string().contains("12"));
        }

        let hidden = FinalMessageSeparator::new(None)
            .with_locale(Locale::EnUs)
            .raw_lines();
        assert!(hidden.is_empty());

        assert_eq!(
            FinalMessageSeparator::new(Some(0))
                .with_locale(Locale::EnUs)
                .raw_lines()[0]
                .to_string(),
            "Worked for <1s"
        );
    }
}
