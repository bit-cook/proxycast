//! Codex-shaped terminal line wrapping with URL-aware heuristics.
//!
//! Wrapping remains a render concern: the canonical Thread/Turn/Item model is
//! never rewritten. The module preserves ratatui span styles while ensuring
//! URLs are not split at `/` or `-`.

use std::borrow::Cow;
use std::ops::Range;

use ratatui::text::{Line, Span};
use textwrap::core::Word;
use textwrap::word_splitters::split_words;
use textwrap::{Options, WordSeparator, WordSplitter};
use unicode_segmentation::UnicodeSegmentation;

use crate::line_truncation::line_width;
use crate::render::line_utils::push_owned_lines;
use crate::width::display_width;

/// Textwrap options using ratatui lines for styled indentation.
#[derive(Debug, Clone)]
pub(crate) struct RtOptions<'a> {
    pub(crate) width: usize,
    pub(crate) line_ending: textwrap::LineEnding,
    pub(crate) initial_indent: Line<'a>,
    pub(crate) subsequent_indent: Line<'a>,
    pub(crate) break_words: bool,
    pub(crate) wrap_algorithm: textwrap::WrapAlgorithm,
    pub(crate) word_separator: textwrap::WordSeparator,
    pub(crate) word_splitter: textwrap::WordSplitter,
}

impl From<usize> for RtOptions<'_> {
    fn from(width: usize) -> Self {
        Self::new(width)
    }
}

impl<'a> RtOptions<'a> {
    pub(crate) fn new(width: usize) -> Self {
        Self {
            width,
            line_ending: textwrap::LineEnding::LF,
            initial_indent: Line::default(),
            subsequent_indent: Line::default(),
            break_words: true,
            wrap_algorithm: textwrap::WrapAlgorithm::FirstFit,
            word_separator: WordSeparator::new(),
            word_splitter: WordSplitter::HyphenSplitter,
        }
    }

    pub(crate) fn initial_indent(self, value: Line<'a>) -> Self {
        Self {
            initial_indent: value,
            ..self
        }
    }

    #[allow(dead_code)]
    pub(crate) fn subsequent_indent(self, value: Line<'a>) -> Self {
        Self {
            subsequent_indent: value,
            ..self
        }
    }

    pub(crate) fn break_words(self, value: bool) -> Self {
        Self {
            break_words: value,
            ..self
        }
    }

    pub(crate) fn word_separator(self, value: WordSeparator) -> Self {
        Self {
            word_separator: value,
            ..self
        }
    }

    #[allow(dead_code)]
    pub(crate) fn wrap_algorithm(self, value: textwrap::WrapAlgorithm) -> Self {
        Self {
            wrap_algorithm: value,
            ..self
        }
    }

    pub(crate) fn word_splitter(self, value: WordSplitter) -> Self {
        Self {
            word_splitter: value,
            ..self
        }
    }
}

/// Projected text keeps source-offset lookup separate from legal grapheme split points.
struct ProjectedText {
    text: String,
    source_boundaries: Vec<(usize, usize)>,
    grapheme_boundaries: Vec<usize>,
}

/// Replaces halfwidth sound-mark graphemes with equally wide, textwrap-safe placeholders.
///
/// Source boundaries recover original byte offsets, while grapheme boundaries keep placeholders
/// indivisible and preserve leading whitespace as a wrapping opportunity.
fn project_halfwidth_sound_marks(text: &str) -> Option<ProjectedText> {
    if !text.contains(['\u{FF9E}', '\u{FF9F}']) {
        return None;
    }

    let mut projected = String::with_capacity(text.len());
    let mut source_boundaries = vec![(0, 0)];
    let mut grapheme_boundaries = vec![0];
    for (source_start, grapheme) in text.grapheme_indices(/*is_extended*/ true) {
        if grapheme.contains(['\u{FF9E}', '\u{FF9F}']) {
            let source_end = source_start + grapheme.len();
            let content_start = grapheme
                .find(|ch: char| !ch.is_whitespace())
                .unwrap_or(grapheme.len());
            let (whitespace, content) = grapheme.split_at(content_start);
            for (offset, ch) in whitespace.char_indices() {
                projected.push(ch);
                source_boundaries.push((projected.len(), source_start + offset + ch.len_utf8()));
                grapheme_boundaries.push(projected.len());
            }

            let width = display_width(content);
            let projected_start = projected.len();
            for _ in 0..width / 2 {
                if projected.len() > projected_start {
                    projected.push('\u{2060}');
                }
                projected.push('界');
                source_boundaries.push((projected.len(), source_end));
            }
            if width % 2 == 1 {
                if projected.len() > projected_start {
                    projected.push('\u{2060}');
                }
                projected.push('a');
                source_boundaries.push((projected.len(), source_end));
            }
        } else {
            for (offset, ch) in grapheme.char_indices() {
                projected.push(ch);
                source_boundaries.push((projected.len(), source_start + offset + ch.len_utf8()));
            }
        }
        grapheme_boundaries.push(projected.len());
    }

    Some(ProjectedText {
        text: projected,
        source_boundaries,
        grapheme_boundaries,
    })
}

/// Maps a projected byte offset back to the corresponding original-text boundary.
fn source_offset(boundaries: &[(usize, usize)], projected_offset: usize) -> usize {
    boundaries
        .binary_search_by_key(&projected_offset, |(offset, _)| *offset)
        .map(|index| boundaries[index].1)
        .unwrap_or(projected_offset)
}

/// Splits oversized projected words without separating placeholders for one source grapheme.
fn break_projected_words<'a>(
    words: impl Iterator<Item = Word<'a>>,
    projected: &'a ProjectedText,
    line_width: usize,
) -> Vec<Word<'a>> {
    let projected_start = projected.text.as_ptr() as usize;
    let mut pieces = Vec::new();

    for word in words {
        if display_width(word.word) <= line_width {
            pieces.push(word);
            continue;
        }

        let word_start = word.word.as_ptr() as usize - projected_start;
        let word_end = word_start + word.word.len();
        let mut piece_start = word_start;
        let mut piece_width = 0;
        let mut atom_start = word_start;
        let boundary_start = projected
            .grapheme_boundaries
            .partition_point(|atom_end| *atom_end <= word_start);

        for atom_end in projected
            .grapheme_boundaries
            .iter()
            .copied()
            .skip(boundary_start)
        {
            if atom_end > word_end {
                break;
            }

            let atom_width = display_width(&projected.text[atom_start..atom_end]);
            if piece_width > 0 && piece_width + atom_width > line_width {
                pieces.push(Word::from(&projected.text[piece_start..atom_start]));
                piece_start = atom_start;
                piece_width = 0;
            }
            piece_width += atom_width;
            atom_start = atom_end;
        }

        let mut last = Word::from(&projected.text[piece_start..word_end]);
        last.whitespace = word.whitespace;
        last.penalty = word.penalty;
        pieces.push(last);
    }

    pieces
}

/// Wraps projected text and translates the resulting ranges back to source byte offsets.
fn wrap_projected_ranges(
    projected: &ProjectedText,
    opts: &Options<'_>,
    include_trailing_spaces: bool,
) -> Vec<Range<usize>> {
    let line_widths = [
        opts.width
            .saturating_sub(display_width(opts.initial_indent)),
        opts.width
            .saturating_sub(display_width(opts.subsequent_indent)),
    ];
    let line_ending = opts.line_ending.as_str();
    let mut ranges = Vec::new();
    let mut line_start = 0;

    for line in projected.text.split(line_ending) {
        let words = opts.word_separator.find_words(line);
        let split_words = split_words(words, &opts.word_splitter);
        let mut broken_words = if opts.break_words {
            break_projected_words(split_words, projected, line_widths[1])
        } else {
            split_words.collect()
        };
        if opts.break_words && !opts.initial_indent.is_empty() {
            broken_words.insert(0, Word::from(""));
        }

        let wrapped_words = opts.wrap_algorithm.wrap(&broken_words, &line_widths);
        let mut cursor = line_start;
        for words in wrapped_words {
            let Some(last_word) = words.last() else {
                let source = source_offset(&projected.source_boundaries, cursor);
                ranges.push(source..source + usize::from(include_trailing_spaces));
                continue;
            };
            let len = words
                .iter()
                .map(|word| word.word.len() + word.whitespace.len())
                .sum::<usize>()
                - last_word.whitespace.len();
            let end = cursor + len;
            let trailing_spaces = if include_trailing_spaces {
                projected.text[end..]
                    .chars()
                    .take_while(|ch| *ch == ' ')
                    .count()
            } else {
                0
            };
            let source_start = source_offset(&projected.source_boundaries, cursor);
            let source_end = source_offset(&projected.source_boundaries, end + trailing_spaces);
            ranges.push(source_start..source_end + usize::from(include_trailing_spaces));
            cursor = end + last_word.whitespace.len();
        }
        line_start += line.len() + line_ending.len();
    }

    ranges
}

/// Return source ranges for wrapped lines, including a one-byte cursor sentinel.
#[allow(dead_code)]
pub(crate) fn wrap_ranges<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    if let Some(projected) = project_halfwidth_sound_marks(text) {
        return wrap_projected_ranges(&projected, &opts, /*include_trailing_spaces*/ true);
    }
    let mut lines: Vec<Range<usize>> = Vec::new();
    let mut cursor = 0usize;
    for (line_index, line) in textwrap::wrap(text, &opts).iter().enumerate() {
        match line {
            std::borrow::Cow::Borrowed(slice) => {
                let range = borrowed_slice_range(text, slice).unwrap_or_else(|| {
                    let synthetic_prefix = if line_index == 0 {
                        opts.initial_indent
                    } else {
                        opts.subsequent_indent
                    };
                    map_owned_wrapped_line_to_range(text, cursor, slice, synthetic_prefix)
                });
                let start = range.start;
                let end = range.end;
                let trailing_spaces = text[end..].chars().take_while(|c| *c == ' ').count();
                lines.push(start..end + trailing_spaces + 1);
                cursor = end + trailing_spaces;
            }
            std::borrow::Cow::Owned(slice) => {
                let synthetic_prefix = if line_index == 0 {
                    opts.initial_indent
                } else {
                    opts.subsequent_indent
                };
                let mapped = map_owned_wrapped_line_to_range(text, cursor, slice, synthetic_prefix);
                let trailing_spaces = text[mapped.end..].chars().take_while(|c| *c == ' ').count();
                lines.push(mapped.start..mapped.end + trailing_spaces + 1);
                cursor = mapped.end + trailing_spaces;
            }
        }
    }
    lines
}

/// Like `wrap_ranges` but returns ranges without trailing whitespace and
/// without the sentinel extra byte. Suitable for general wrapping where
/// trailing spaces should not be preserved.
pub(crate) fn wrap_ranges_trim<'a, O>(text: &str, width_or_options: O) -> Vec<Range<usize>>
where
    O: Into<Options<'a>>,
{
    let opts = width_or_options.into();
    if let Some(projected) = project_halfwidth_sound_marks(text) {
        return wrap_projected_ranges(&projected, &opts, /*include_trailing_spaces*/ false);
    }
    let mut lines: Vec<Range<usize>> = Vec::new();
    let mut cursor = 0usize;
    for (line_index, line) in textwrap::wrap(text, &opts).iter().enumerate() {
        match line {
            std::borrow::Cow::Borrowed(slice) => {
                let range = borrowed_slice_range(text, slice).unwrap_or_else(|| {
                    let synthetic_prefix = if line_index == 0 {
                        opts.initial_indent
                    } else {
                        opts.subsequent_indent
                    };
                    map_owned_wrapped_line_to_range(text, cursor, slice, synthetic_prefix)
                });
                cursor = range.end;
                lines.push(range);
            }
            std::borrow::Cow::Owned(slice) => {
                let synthetic_prefix = if line_index == 0 {
                    opts.initial_indent
                } else {
                    opts.subsequent_indent
                };
                let mapped = map_owned_wrapped_line_to_range(text, cursor, slice, synthetic_prefix);
                lines.push(mapped.clone());
                cursor = mapped.end;
            }
        }
    }
    lines
}

fn borrowed_slice_range(text: &str, slice: &str) -> Option<Range<usize>> {
    let text_start = text.as_ptr() as usize;
    let text_end = text_start.checked_add(text.len())?;
    let slice_start = slice.as_ptr() as usize;
    let slice_end = slice_start.checked_add(slice.len())?;

    if slice_start < text_start || slice_end > text_end {
        return None;
    }

    Some((slice_start - text_start)..(slice_end - text_start))
}

/// Maps an owned (materialized) wrapped line back to a byte range in `text`.
fn map_owned_wrapped_line_to_range(
    text: &str,
    cursor: usize,
    wrapped: &str,
    synthetic_prefix: &str,
) -> Range<usize> {
    let wrapped = if synthetic_prefix.is_empty() {
        wrapped
    } else {
        wrapped.strip_prefix(synthetic_prefix).unwrap_or(wrapped)
    };

    let mut start = cursor;
    while start < text.len() && !wrapped.starts_with(' ') {
        let Some(ch) = text[start..].chars().next() else {
            break;
        };
        if ch != ' ' {
            break;
        }
        start += ch.len_utf8();
    }

    let mut end = start;
    let mut saw_source_char = false;
    let mut chars = wrapped.chars().peekable();
    while let Some(ch) = chars.next() {
        if end < text.len() {
            let Some(src) = text[end..].chars().next() else {
                unreachable!("checked end < text.len()");
            };
            if ch == src {
                end += src.len_utf8();
                saw_source_char = true;
                continue;
            }
        }

        // textwrap can materialize owned lines when penalties are inserted.
        // The default penalty is a trailing '-'; it does not correspond to
        // source bytes, so we skip it while keeping byte ranges in source text.
        if ch == '-' && chars.peek().is_none() {
            continue;
        }

        // Non-source chars can be synthesized by textwrap in owned output
        // (e.g. non-space indent prefixes). Keep going and map the source bytes
        // we can confidently match instead of crashing the app.
        if !saw_source_char {
            continue;
        }

        tracing::warn!(
            wrapped = %wrapped,
            cursor,
            end,
            "wrap_ranges: could not fully map owned line; returning partial source range"
        );
        break;
    }

    start..end
}

/// Wraps a single ratatui `Line`, automatically switching to URL-preserving
/// options when the line contains a URL-like token.
#[must_use]
pub(crate) fn adaptive_wrap_line<'a>(line: &'a Line<'a>, base: RtOptions<'a>) -> Vec<Line<'a>> {
    let (flat, span_bounds) = flatten_line(line);
    let mut saw_url = false;
    let mut saw_non_url = false;

    for token in flat.split_ascii_whitespace() {
        if is_url_like_token(token) {
            saw_url = true;
        } else if is_substantive_non_url_token(token) {
            saw_non_url = true;
        }

        if saw_url && saw_non_url {
            break;
        }
    }

    if !saw_url {
        word_wrap_flattened_line(line, &flat, &span_bounds, base)
    } else if saw_non_url {
        mixed_url_wrap_line(line, &flat, &span_bounds, base)
    } else {
        word_wrap_flattened_line(line, &flat, &span_bounds, url_preserving_wrap_options(base))
    }
}

/// Wraps multiple input lines with URL-aware heuristics, applying
/// `initial_indent` to the first line and `subsequent_indent` to the rest.
#[allow(private_bounds)]
pub(crate) fn adaptive_wrap_lines<'a, I, L>(
    lines: I,
    width_or_options: RtOptions<'a>,
) -> Vec<Line<'static>>
where
    I: IntoIterator<Item = L>,
    L: IntoLineInput<'a>,
{
    let base_opts = width_or_options;
    let mut out = Vec::new();

    for (idx, line) in lines.into_iter().enumerate() {
        let line_input = line.into_line_input();
        let opts = if idx == 0 {
            base_opts.clone()
        } else {
            base_opts
                .clone()
                .initial_indent(base_opts.subsequent_indent.clone())
        };

        let wrapped = adaptive_wrap_line(line_input.as_ref(), opts);
        push_owned_lines(&wrapped, &mut out);
    }

    out
}

#[must_use]
pub(crate) fn word_wrap_line<'a, O>(line: &'a Line<'a>, width_or_options: O) -> Vec<Line<'a>>
where
    O: Into<RtOptions<'a>>,
{
    let (flat, span_bounds) = flatten_line(line);
    word_wrap_flattened_line(line, &flat, &span_bounds, width_or_options.into())
}

fn word_wrap_flattened_line<'a>(
    line: &'a Line<'a>,
    flat: &str,
    span_bounds: &[(Range<usize>, ratatui::style::Style)],
    rt_opts: RtOptions<'a>,
) -> Vec<Line<'a>> {
    let opts = Options::new(rt_opts.width)
        .line_ending(rt_opts.line_ending)
        .break_words(rt_opts.break_words)
        .wrap_algorithm(rt_opts.wrap_algorithm)
        .word_separator(rt_opts.word_separator)
        .word_splitter(rt_opts.word_splitter);

    let initial_width_available = opts
        .width
        .saturating_sub(line_width(&rt_opts.initial_indent))
        .max(1);
    let initial_wrapped = wrap_ranges_trim(flat, opts.clone().width(initial_width_available));
    let Some(first_line_range) = initial_wrapped.first() else {
        return vec![rt_opts.initial_indent.clone()];
    };

    let mut out = Vec::new();
    let mut first_line = rt_opts.initial_indent.clone().style(line.style);
    {
        let sliced = slice_line_spans(line, span_bounds, first_line_range);
        let mut spans = first_line.spans;
        spans.extend(sliced.spans.into_iter().map(|s| s.patch_style(line.style)));
        first_line.spans = spans;
        out.push(first_line);
    }

    let base = first_line_range.end;
    let skip_leading_spaces = flat[base..].chars().take_while(|c| *c == ' ').count();
    let base = base + skip_leading_spaces;
    let subsequent_width_available = opts
        .width
        .saturating_sub(line_width(&rt_opts.subsequent_indent))
        .max(1);
    let remaining_wrapped = wrap_ranges_trim(&flat[base..], opts.width(subsequent_width_available));
    for range in &remaining_wrapped {
        if range.is_empty() {
            continue;
        }
        let mut subsequent_line = rt_opts.subsequent_indent.clone().style(line.style);
        let offset_range = (range.start + base)..(range.end + base);
        let sliced = slice_line_spans(line, span_bounds, &offset_range);
        let mut spans = subsequent_line.spans;
        spans.extend(sliced.spans.into_iter().map(|s| s.patch_style(line.style)));
        subsequent_line.spans = spans;
        out.push(subsequent_line);
    }

    out
}

#[derive(Clone, Debug)]
struct MixedUrlWord {
    range: Range<usize>,
    is_url: bool,
}

impl MixedUrlWord {
    fn width(&self, text: &str) -> usize {
        display_width(&text[self.range.clone()])
    }
}

fn mixed_url_wrap_line<'a>(
    line: &'a Line<'a>,
    flat: &str,
    span_bounds: &[(Range<usize>, ratatui::style::Style)],
    rt_opts: RtOptions<'a>,
) -> Vec<Line<'a>> {
    let initial_width_available = rt_opts
        .width
        .saturating_sub(line_width(&rt_opts.initial_indent))
        .max(1);
    let subsequent_width_available = rt_opts
        .width
        .saturating_sub(line_width(&rt_opts.subsequent_indent))
        .max(1);
    let ranges = mixed_url_wrap_ranges(flat, initial_width_available, subsequent_width_available);

    let mut out = Vec::new();
    for (idx, range) in ranges.iter().enumerate() {
        let mut wrapped_line = if idx == 0 {
            rt_opts.initial_indent.clone()
        } else {
            rt_opts.subsequent_indent.clone()
        }
        .style(line.style);
        let sliced = slice_line_spans(line, span_bounds, range);
        let mut spans = wrapped_line.spans;
        spans.extend(
            sliced
                .spans
                .into_iter()
                .map(|span| span.patch_style(line.style)),
        );
        wrapped_line.spans = spans;
        out.push(wrapped_line);
    }

    if out.is_empty() {
        vec![rt_opts.initial_indent.clone()]
    } else {
        out
    }
}

fn mixed_url_wrap_ranges(
    text: &str,
    initial_width: usize,
    subsequent_width: usize,
) -> Vec<Range<usize>> {
    let leading_space_width = text.chars().take_while(|ch| *ch == ' ').count();
    let mut words = Vec::new();
    let mut cursor = 0usize;
    for word in WordSeparator::AsciiSpace.find_words(text) {
        let word_start = cursor;
        let word_end = word_start + word.word.len();
        let trailing_space_end = word_end + word.whitespace.len();
        if !word.word.is_empty() {
            words.push(MixedUrlWord {
                range: word_start..word_end,
                is_url: is_url_like_token(word.word),
            });
        }
        cursor = trailing_space_end;
    }

    let mut lines = Vec::new();
    let mut line_start = None;
    let mut line_end = 0usize;
    let mut line_width = 0usize;
    let mut line_limit = initial_width.max(1);

    for word in words {
        let mut pending = split_mixed_url_word(text, word, line_limit);
        let mut pending_idx = 0usize;

        while let Some(piece) = pending.get(pending_idx).cloned() {
            let empty_line_prefix_width = if line_start.is_none() && lines.is_empty() {
                leading_space_width
            } else {
                0
            };
            let empty_line_piece_limit = line_limit.saturating_sub(empty_line_prefix_width).max(1);
            let mut indivisible = false;
            if line_start.is_none() && !piece.is_url && piece.width(text) > empty_line_piece_limit {
                let split = split_mixed_url_word(text, piece.clone(), empty_line_piece_limit);
                if split.len() > 1 {
                    pending.splice(pending_idx..=pending_idx, split);
                    continue;
                }
                indivisible = true;
            }

            let piece_width = piece.width(text);
            let inter_word_space = line_start
                .map(|_| text[line_end..piece.range.start].len())
                .unwrap_or(0);
            let fits = if line_start.is_none() {
                piece.is_url
                    || indivisible
                    || empty_line_prefix_width + piece_width <= line_limit
                    || empty_line_prefix_width >= line_limit
            } else {
                line_width + inter_word_space + piece_width <= line_limit
            };

            if fits {
                if line_start.is_none() {
                    let is_first_output_line = lines.is_empty();
                    let start = if is_first_output_line {
                        0
                    } else {
                        piece.range.start
                    };
                    line_start = Some(start);
                    line_width = if is_first_output_line {
                        leading_space_width + piece_width
                    } else {
                        piece_width
                    };
                } else {
                    line_width += inter_word_space + piece_width;
                }
                line_end = piece.range.end;
                pending_idx += 1;
                continue;
            }

            if let Some(start) = line_start.take() {
                lines.push(start..line_end);
            }
            line_end = 0;
            line_width = 0;
            line_limit = subsequent_width.max(1);
        }
    }

    if let Some(start) = line_start {
        lines.push(start..line_end);
    }

    lines
}

fn split_mixed_url_word(text: &str, word: MixedUrlWord, line_limit: usize) -> Vec<MixedUrlWord> {
    if word.is_url || word.width(text) <= line_limit {
        return vec![word];
    }

    let mut pieces = Vec::new();
    let mut start = word.range.start;
    let mut width = 0usize;
    for (offset, grapheme) in text[word.range.clone()].grapheme_indices(/*is_extended*/ true) {
        let grapheme_width = display_width(grapheme);
        if width > 0 && width + grapheme_width > line_limit.max(1) {
            let end = word.range.start + offset;
            pieces.push(MixedUrlWord {
                range: start..end,
                is_url: false,
            });
            start = end;
            width = 0;
        }
        width += grapheme_width;
    }
    if start < word.range.end {
        pieces.push(MixedUrlWord {
            range: start..word.range.end,
            is_url: false,
        });
    }
    pieces
}

fn flatten_line(line: &Line<'_>) -> (String, Vec<(Range<usize>, ratatui::style::Style)>) {
    let mut text = String::new();
    let mut spans = Vec::new();
    let mut cursor = 0;
    for span in &line.spans {
        let start = cursor;
        text.push_str(span.content.as_ref());
        cursor += span.content.len();
        spans.push((start..cursor, span.style));
    }
    (text, spans)
}

fn slice_line_spans<'a>(
    original: &'a Line<'a>,
    bounds: &[(Range<usize>, ratatui::style::Style)],
    requested: &Range<usize>,
) -> Line<'a> {
    let mut output = Vec::new();
    for (index, (bound, style)) in bounds.iter().enumerate() {
        if bound.end <= requested.start {
            continue;
        }
        if bound.start >= requested.end {
            break;
        }
        let start = bound.start.max(requested.start) - bound.start;
        let end = bound.end.min(requested.end) - bound.start;
        if start < end {
            let content = original.spans[index].content.as_ref();
            output.push(Span {
                style: *style,
                content: Cow::Borrowed(&content[start..end]),
            });
        }
    }
    Line {
        style: original.style,
        alignment: original.alignment,
        spans: output,
    }
}

/// Wrap a sequence of lines and return owned output suitable for a transcript.
#[allow(dead_code, private_bounds)]
pub(crate) fn word_wrap_lines<'a, I, O, L>(lines: I, options: O) -> Vec<Line<'static>>
where
    I: IntoIterator<Item = L>,
    L: IntoLineInput<'a>,
    O: Into<RtOptions<'a>>,
{
    let base_opts: RtOptions<'a> = options.into();
    let mut out = Vec::new();

    for (idx, line) in lines.into_iter().enumerate() {
        let line_input = line.into_line_input();
        let opts = if idx == 0 {
            base_opts.clone()
        } else {
            base_opts
                .clone()
                .initial_indent(base_opts.subsequent_indent.clone())
        };
        let wrapped = word_wrap_line(line_input.as_ref(), opts);
        push_owned_lines(&wrapped, &mut out);
    }

    out
}

fn owned_line(line: &Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .iter()
            .map(|span| Span::styled(span.content.to_string(), span.style))
            .collect(),
    }
}

pub(crate) fn own_lines<'a>(lines: Vec<Line<'a>>) -> Vec<Line<'static>> {
    lines.iter().map(owned_line).collect()
}

#[derive(Debug)]
enum LineInput<'a> {
    Borrowed(&'a Line<'a>),
    Owned(Line<'a>),
}

impl<'a> LineInput<'a> {
    fn as_ref(&self) -> &Line<'a> {
        match self {
            Self::Borrowed(line) => line,
            Self::Owned(line) => line,
        }
    }
}

#[allow(dead_code)]
trait IntoLineInput<'a> {
    fn into_line_input(self) -> LineInput<'a>;
}

impl<'a> IntoLineInput<'a> for &'a Line<'a> {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Borrowed(self)
    }
}

#[allow(dead_code)]
impl<'a> IntoLineInput<'a> for &'a mut Line<'a> {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Borrowed(self)
    }
}

impl<'a> IntoLineInput<'a> for Line<'a> {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Owned(self)
    }
}

impl<'a> IntoLineInput<'a> for &'a str {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Owned(Line::from(self))
    }
}

impl<'a> IntoLineInput<'a> for String {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Owned(Line::from(self))
    }
}

#[allow(dead_code)]
impl<'a> IntoLineInput<'a> for Cow<'a, str> {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Owned(Line::from(self))
    }
}

#[allow(dead_code)]
impl<'a> IntoLineInput<'a> for Span<'a> {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Owned(Line::from(self))
    }
}

#[allow(dead_code)]
impl<'a> IntoLineInput<'a> for Vec<Span<'a>> {
    fn into_line_input(self) -> LineInput<'a> {
        LineInput::Owned(Line::from(self))
    }
}

/// Reconfigures wrapping options so that URL-like tokens are never split.
pub(crate) fn url_preserving_wrap_options<'a>(opts: RtOptions<'a>) -> RtOptions<'a> {
    opts.word_separator(WordSeparator::AsciiSpace)
        .word_splitter(WordSplitter::NoHyphenation)
        .break_words(false)
}

/// Returns `true` if any whitespace-delimited token in `line` looks like a URL.
pub(crate) fn line_contains_url_like(line: &Line<'_>) -> bool {
    let text: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    text_contains_url_like(&text)
}

/// Returns `true` if `line` contains both a URL-like token and at least one
/// substantive non-URL token.
#[allow(dead_code)]
pub(crate) fn line_has_mixed_url_and_non_url_tokens(line: &Line<'_>) -> bool {
    let text: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    text_has_mixed_url_and_non_url_tokens(&text)
}

/// Returns `true` if any whitespace-delimited token in `text` looks like a URL.
pub(crate) fn text_contains_url_like(text: &str) -> bool {
    text.split_ascii_whitespace().any(is_url_like_token)
}

fn text_has_mixed_url_and_non_url_tokens(text: &str) -> bool {
    let mut saw_url = false;
    let mut saw_non_url = false;

    for raw_token in text.split_ascii_whitespace() {
        if is_url_like_token(raw_token) {
            saw_url = true;
        } else if is_substantive_non_url_token(raw_token) {
            saw_non_url = true;
        }

        if saw_url && saw_non_url {
            return true;
        }
    }

    false
}

fn is_url_like_token(raw_token: &str) -> bool {
    let token = trim_url_token(raw_token);
    !token.is_empty() && (is_absolute_url_like(token) || is_bare_url_like(token))
}

fn is_substantive_non_url_token(raw_token: &str) -> bool {
    let token = trim_url_token(raw_token);
    if token.is_empty() || is_decorative_marker_token(raw_token, token) {
        return false;
    }

    token.chars().any(char::is_alphanumeric)
}

fn is_decorative_marker_token(raw_token: &str, token: &str) -> bool {
    let raw = raw_token.trim();
    matches!(
        raw,
        "-" | "*"
            | "+"
            | "•"
            | "◦"
            | "▪"
            | ">"
            | "|"
            | "│"
            | "┆"
            | "└"
            | "├"
            | "┌"
            | "┐"
            | "┘"
            | "┼"
    ) || is_ordered_list_marker(raw, token)
}

fn is_ordered_list_marker(raw_token: &str, token: &str) -> bool {
    token.chars().all(|c| c.is_ascii_digit())
        && (raw_token.ends_with('.') || raw_token.ends_with(')'))
}

fn trim_url_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        matches!(
            c,
            '(' | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | ','
                | '.'
                | ';'
                | ':'
                | '!'
                | '\''
                | '"'
        )
    })
}

fn is_absolute_url_like(token: &str) -> bool {
    if !token.contains("://") {
        return false;
    }

    if let Ok(url) = url::Url::parse(token) {
        let scheme = url.scheme().to_ascii_lowercase();
        if matches!(
            scheme.as_str(),
            "http" | "https" | "ftp" | "ftps" | "ws" | "wss"
        ) {
            return url.host_str().is_some();
        }
        return true;
    }

    has_valid_scheme_prefix(token)
}

fn has_valid_scheme_prefix(token: &str) -> bool {
    let Some((scheme, rest)) = token.split_once("://") else {
        return false;
    };
    if scheme.is_empty() || rest.is_empty() {
        return false;
    }

    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
}

fn is_bare_url_like(token: &str) -> bool {
    let (host_port, has_trailer) = split_host_port_and_trailer(token);
    if host_port.is_empty() {
        return false;
    }

    if !has_trailer && !host_port.to_ascii_lowercase().starts_with("www.") {
        return false;
    }

    let (host, port) = split_host_and_port(host_port);
    if host.is_empty() {
        return false;
    }
    if let Some(port) = port {
        if !is_valid_port(port) {
            return false;
        }
    }

    host.eq_ignore_ascii_case("localhost") || is_ipv4(host) || is_domain_name(host)
}

fn split_host_port_and_trailer(token: &str) -> (&str, bool) {
    if let Some(idx) = token.find(['/', '?', '#']) {
        (&token[..idx], true)
    } else {
        (token, false)
    }
}

fn split_host_and_port(host_port: &str) -> (&str, Option<&str>) {
    if host_port.starts_with('[') {
        return (host_port, None);
    }

    if let Some((host, port)) = host_port.rsplit_once(':') {
        if !host.is_empty() && !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            return (host, Some(port));
        }
    }

    (host_port, None)
}

fn is_valid_port(port: &str) -> bool {
    if port.is_empty() || port.len() > 5 || !port.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    port.parse::<u16>().is_ok()
}

fn is_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    parts
        .iter()
        .all(|part| !part.is_empty() && part.parse::<u8>().is_ok())
}

fn is_domain_name(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    if !host.contains('.') {
        return false;
    }

    let mut labels = host.split('.');
    let Some(tld) = labels.next_back() else {
        return false;
    };
    if !is_tld(tld) {
        return false;
    }

    labels.all(is_domain_label)
}

fn is_tld(label: &str) -> bool {
    (2..=63).contains(&label.len()) && label.chars().all(|c| c.is_ascii_alphabetic())
}

fn is_domain_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }

    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let Some(last) = label.chars().next_back() else {
        return false;
    };

    first.is_ascii_alphanumeric()
        && last.is_ascii_alphanumeric()
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Stylize;
    use ratatui::style::{Color, Style};

    fn concat_line(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn rendered(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn word_wrap_preserves_styled_spans_and_indent() {
        let line = Line::from(vec![
            Span::styled("hello ", Style::default().fg(Color::Red)),
            Span::styled("world", Style::default().fg(Color::Green)),
        ]);
        let wrapped = word_wrap_line(&line, RtOptions::new(8).subsequent_indent(Line::from("  ")));
        assert_eq!(rendered(&wrapped), "hello\n  world");
        assert_eq!(wrapped[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(
            wrapped[1].spans.last().unwrap().style.fg,
            Some(Color::Green)
        );
    }

    #[test]
    fn adaptive_wrap_keeps_url_token_intact() {
        let line = Line::from("see https://example.com/a-long/path now");
        let wrapped = adaptive_wrap_line(&line, RtOptions::new(18));
        let text = rendered(&wrapped);
        assert!(text.contains("https://example.com/a-long/path"));
        assert!(line_contains_url_like(&line));
    }

    #[test]
    fn url_detection_rejects_file_paths() {
        assert!(text_contains_url_like("open https://example.com")
            .then_some(())
            .is_some());
        assert!(!text_contains_url_like("src/main.rs foo/bar"));
    }

    #[test]
    fn wrap_ranges_are_utf8_safe() {
        let text = "你好 世界";
        let ranges = wrap_ranges_trim(text, Options::new(5));
        assert_eq!(
            ranges
                .iter()
                .map(|range| &text[range.clone()])
                .collect::<Vec<_>>(),
            vec!["你好", "世界"]
        );
        assert!(wrap_ranges(text, Options::new(5))
            .iter()
            .all(|range| range.end <= text.len() + 1));
    }

    #[test]
    fn multiline_wrap_returns_owned_lines() {
        let lines = word_wrap_lines(["one two", "three"], 5usize);
        assert_eq!(rendered(&lines), "one\ntwo\nthree");
        assert!(lines.iter().all(|line| {
            line.spans
                .iter()
                .all(|span| matches!(span.content, Cow::Owned(_)))
        }));
    }

    #[test]
    fn trivial_unstyled_no_indents_wide_width() {
        let line = Line::from("hello");
        let out = word_wrap_line(&line, 10);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "hello");
    }

    #[test]
    fn simple_unstyled_wrap_narrow_width() {
        let line = Line::from("hello world");
        let out = word_wrap_line(&line, 5);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn simple_styled_wrap_preserves_styles() {
        let line = Line::from(vec!["hello ".red(), "world".into()]);
        let out = word_wrap_line(&line, 6);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(out[0].spans.len(), 1);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(concat_line(&out[1]), "world");
        assert_eq!(out[1].spans.len(), 1);
        assert_eq!(out[1].spans[0].style.fg, None);
    }

    #[test]
    fn with_initial_and_subsequent_indents() {
        let opts = RtOptions::new(8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));
        let line = Line::from("hello world foo");
        let out = word_wrap_line(&line, opts);
        assert_eq!(concat_line(&out[0]), "- hello");
        assert_eq!(concat_line(&out[1]), "  world");
        assert_eq!(concat_line(&out[2]), "  foo");
    }

    #[test]
    fn empty_initial_indent_subsequent_spaces() {
        let opts = RtOptions::new(8)
            .initial_indent(Line::from(""))
            .subsequent_indent(Line::from("    "));
        let line = Line::from("hello world foobar");
        let out = word_wrap_line(&line, opts);
        assert!(concat_line(&out[0]).starts_with("hello"));
        for line in &out[1..] {
            assert!(concat_line(line).starts_with("    "));
        }
    }

    #[test]
    fn empty_input_yields_single_empty_line() {
        let line = Line::from("");
        let out = word_wrap_line(&line, 10);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "");
    }

    #[test]
    fn leading_spaces_preserved_on_first_line() {
        let line = Line::from("   hello");
        let out = word_wrap_line(&line, 8);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "   hello");
    }

    #[test]
    fn multiple_spaces_between_words_dont_start_next_line_with_spaces() {
        let line = Line::from("hello   world");
        let out = word_wrap_line(&line, 8);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn break_words_false_allows_overflow_for_long_word() {
        let opts = RtOptions::new(5).break_words(false);
        let line = Line::from("supercalifragilistic");
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), "supercalifragilistic");
    }

    #[test]
    fn hyphen_splitter_breaks_at_hyphen() {
        let line = Line::from("hello-world");
        let out = word_wrap_line(&line, 7);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "hello-");
        assert_eq!(concat_line(&out[1]), "world");
    }

    #[test]
    fn indent_consumes_width_leaving_one_char_space() {
        let opts = RtOptions::new(4)
            .initial_indent(Line::from(">>>>"))
            .subsequent_indent(Line::from("--"));
        let line = Line::from("hello");
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 3);
        assert_eq!(concat_line(&out[0]), ">>>>h");
        assert_eq!(concat_line(&out[1]), "--el");
        assert_eq!(concat_line(&out[2]), "--lo");
    }

    #[test]
    fn wide_unicode_wraps_by_display_width() {
        let line = Line::from("😀😀😀");
        let out = word_wrap_line(&line, 4);
        assert_eq!(out.len(), 2);
        assert_eq!(concat_line(&out[0]), "😀😀");
        assert_eq!(concat_line(&out[1]), "😀");
    }

    #[test]
    fn styled_split_within_span_preserves_style() {
        let line = Line::from(vec!["abcd".red()]);
        let out = word_wrap_line(&line, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].spans.len(), 1);
        assert_eq!(out[1].spans.len(), 1);
        assert_eq!(out[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(out[1].spans[0].style.fg, Some(Color::Red));
        assert_eq!(concat_line(&out[0]), "ab");
        assert_eq!(concat_line(&out[1]), "cd");
    }

    #[test]
    fn wrap_lines_applies_initial_indent_only_once() {
        let opts = RtOptions::new(8)
            .initial_indent(Line::from("- "))
            .subsequent_indent(Line::from("  "));
        let lines = vec![Line::from("hello world"), Line::from("foo bar baz")];
        let out = word_wrap_lines(lines, opts);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert!(rendered[0].starts_with("- "));
        for row in rendered.iter().skip(1) {
            assert!(row.starts_with("  "));
        }
    }

    #[test]
    fn wrap_lines_without_indents_is_concat_of_single_wraps() {
        let lines = vec![Line::from("hello"), Line::from("world!")];
        let out = word_wrap_lines(lines, 10);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello", "world!"]);
    }

    #[test]
    fn wrap_lines_accepts_borrowed_iterators() {
        let lines = [Line::from("hello world"), Line::from("foo bar baz")];
        let out = word_wrap_lines(lines, 10);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello", "world", "foo bar", "baz"]);
    }

    #[test]
    fn wrap_lines_accepts_str_slices() {
        let lines = ["hello world", "goodnight moon"];
        let out = word_wrap_lines(lines, 12);
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, vec!["hello world", "goodnight", "moon"]);
    }

    #[test]
    fn line_height_counts_double_width_emoji() {
        let line = Line::from("😀😀😀");
        assert_eq!(word_wrap_line(&line, 4).len(), 2);
        assert_eq!(word_wrap_line(&line, 2).len(), 3);
        assert_eq!(word_wrap_line(&line, 6).len(), 1);
    }

    #[test]
    fn word_wrap_does_not_split_words_simple_english() {
        let sample = "Years passed, and Willowmere thrived in peace and friendship. Mira’s herb garden flourished with both ordinary and enchanted plants, and travelers spoke of the kindness of the woman who tended them.";
        let lines = [Line::from(sample)];
        let wrapped = word_wrap_lines(&lines, 40);
        let joined = wrapped
            .iter()
            .map(concat_line)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            joined,
            "Years passed, and Willowmere thrived in\npeace and friendship. Mira’s herb garden\nflourished with both ordinary and\nenchanted plants, and travelers spoke of\nthe kindness of the woman who tended\nthem."
        );
    }

    #[test]
    fn ascii_space_separator_with_no_hyphenation_keeps_url_intact() {
        let line = Line::from(
            "http://example.com/long-url-with-dashes-wider-than-terminal-window/blah-blah-blah-text/more-gibberish-text",
        );
        let opts = RtOptions::new(24)
            .word_separator(WordSeparator::AsciiSpace)
            .word_splitter(WordSplitter::NoHyphenation)
            .break_words(false);
        let out = word_wrap_line(&line, opts);
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), concat_line(&line));
    }

    #[test]
    fn text_contains_url_like_matches_expected_tokens() {
        for text in [
            "https://example.com/a/b",
            "ftp://host/path",
            "www.example.com/path?x=1",
            "example.test/path#frag",
            "localhost:3000/api",
            "127.0.0.1:8080/health",
            "(https://example.com/wrapped-in-parens)",
        ] {
            assert!(
                text_contains_url_like(text),
                "expected URL-like match for {text:?}"
            );
        }
    }

    #[test]
    fn text_contains_url_like_rejects_non_urls() {
        for text in [
            "src/main.rs",
            "foo/bar",
            "key:value",
            "just-some-text-with-dashes",
            "hello.world",
        ] {
            assert!(
                !text_contains_url_like(text),
                "did not expect URL-like match for {text:?}"
            );
        }
    }

    #[test]
    fn line_contains_url_like_checks_across_spans() {
        let line = Line::from(vec![
            "see ".into(),
            "https://example.com/a/very/long/path".cyan(),
            " for details".into(),
        ]);
        assert!(line_contains_url_like(&line));
    }

    #[test]
    fn line_has_mixed_url_and_non_url_tokens_detects_prose_plus_url() {
        let line = Line::from("see https://example.com/path for details");
        assert!(line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn line_has_mixed_url_and_non_url_tokens_ignores_pipe_prefix() {
        let line = Line::from(vec!["  │ ".into(), "https://example.com/path".into()]);
        assert!(!line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn line_has_mixed_url_and_non_url_tokens_ignores_ordered_list_marker() {
        let line = Line::from("1. https://example.com/path");
        assert!(!line_has_mixed_url_and_non_url_tokens(&line));
    }

    #[test]
    fn text_contains_url_like_accepts_custom_scheme_with_separator() {
        assert!(text_contains_url_like("myapp://open/some/path"));
    }

    #[test]
    fn text_contains_url_like_rejects_invalid_ports() {
        assert!(!text_contains_url_like("localhost:99999/path"));
        assert!(!text_contains_url_like("example.com:abc/path"));
    }

    #[test]
    fn adaptive_wrap_line_keeps_long_url_like_token_intact() {
        let line = Line::from("example.test/a-very-long-path-with-many-segments-and-query?x=1&y=2");
        let out = adaptive_wrap_line(&line, RtOptions::new(20));
        assert_eq!(out.len(), 1);
        assert_eq!(concat_line(&out[0]), concat_line(&line));
    }

    #[test]
    fn adaptive_wrap_line_preserves_default_behavior_for_non_url_tokens() {
        let line = Line::from("a_very_long_token_without_spaces_to_force_wrapping");
        let out = adaptive_wrap_line(&line, RtOptions::new(20));
        assert!(out.len() > 1);
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_keeps_regular_words_intact() {
        let line = Line::from(
            "see https://example.com/path and keep strikethrough intact while wrapping prose",
        );
        let out = adaptive_wrap_line(&line, RtOptions::new(36));
        let joined = out.iter().map(concat_line).collect::<Vec<_>>().join("\n");
        assert_eq!(
            joined,
            "see https://example.com/path and\nkeep strikethrough intact while\nwrapping prose"
        );
    }

    #[test]
    fn adaptive_wrap_line_keeps_url_split_across_styled_spans_intact() {
        let line = Line::from(vec![
            "see ".red(),
            "https://exa".cyan(),
            "mple.com/path".magenta(),
            " now".green(),
        ]);
        let out = adaptive_wrap_line(&line, RtOptions::new(10));
        assert_eq!(
            out,
            vec![
                Line::from("see".red()),
                Line::from(vec!["https://exa".cyan(), "mple.com/path".magenta()]),
                Line::from("now".green()),
            ]
        );
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_wraps_long_non_url_token() {
        let long_non_url = "a_very_long_token_without_spaces_to_force_wrapping";
        let line = Line::from(format!("see https://ex.com {long_non_url}"));
        let out = adaptive_wrap_line(&line, RtOptions::new(24));
        assert!(out
            .iter()
            .any(|line| concat_line(line).contains("https://ex.com")));
        assert!(!out
            .iter()
            .any(|line| concat_line(line).contains(long_non_url)));
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_counts_leading_spaces_before_first_word() {
        let line = Line::from("      abcdefgh https://x.co");
        let out = adaptive_wrap_line(&line, RtOptions::new(10).subsequent_indent("      ".into()));
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(
            rendered[..2],
            ["      abcd".to_string(), "      efgh".to_string()]
        );
    }

    #[test]
    fn adaptive_wrap_line_mixed_line_resplits_long_token_for_continuation_width() {
        let line = Line::from("abcdefghijklmnopqrst https://x.co");
        let out = adaptive_wrap_line(&line, RtOptions::new(10).subsequent_indent("    ".into()));
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(
            rendered[..3],
            [
                "abcdefghij".to_string(),
                "    klmnop".to_string(),
                "    qrst".to_string(),
            ]
        );
    }

    #[test]
    fn adaptive_wrap_line_mixed_url_counts_halfwidth_sound_marks() {
        let line = Line::from("ｶﾞﾊﾟtail https://x.co");
        let out = adaptive_wrap_line(&line, RtOptions::new(4));
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, ["ｶﾞﾊﾟ", "tail", "https://x.co"]);
    }

    #[test]
    fn adaptive_wrap_line_mixed_url_makes_progress_for_an_indivisible_grapheme() {
        let line = Line::from("ｶﾞ https://x.co");
        let out = adaptive_wrap_line(&line, RtOptions::new(1));
        let rendered: Vec<String> = out.iter().map(concat_line).collect();
        assert_eq!(rendered, ["ｶﾞ", "https://x.co"]);
    }

    #[test]
    fn map_owned_wrapped_line_to_range_recovers_on_non_prefix_mismatch() {
        let range = map_owned_wrapped_line_to_range("hello world", 0, "helloX", "");
        assert_eq!(range, 0..5);
    }

    #[test]
    fn borrowed_slice_range_rejects_slices_outside_source_text() {
        let text = "test message";
        let external = String::from("test");
        assert_eq!(borrowed_slice_range(text, &external), None);
        assert_eq!(
            map_owned_wrapped_line_to_range(text, 0, &external, ""),
            0..4
        );
    }

    #[test]
    fn map_owned_wrapped_line_to_range_indent_coincides_with_source() {
        let text = "- item one and some more words";
        let range = map_owned_wrapped_line_to_range(text, 0, "- - item one", "- ");
        assert_eq!(range, 0..10);
    }

    #[test]
    fn wrap_ranges_indent_prefix_coincides_with_source_char() {
        let text = "- first item is long enough to wrap around";
        let opts = || {
            textwrap::Options::new(16)
                .initial_indent("- ")
                .subsequent_indent("- ")
        };
        let ranges = wrap_ranges(text, opts());
        assert!(!ranges.is_empty());

        let mut rebuilt = String::new();
        let mut cursor = 0usize;
        for range in ranges {
            let start = range.start.max(cursor).min(text.len());
            let end = range.end.min(text.len());
            if start < end {
                rebuilt.push_str(&text[start..end]);
            }
            cursor = cursor.max(end);
        }
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn wrap_ranges_count_halfwidth_sound_marks_without_changing_byte_offsets() {
        for (text, width, expected) in [
            ("abｶﾞc", 4, vec!["abｶﾞ", "c"]),
            ("abｶﾞc", 3, vec!["ab", "ｶﾞc"]),
            ("ﾞab", 2, vec!["ﾞa", "b"]),
            ("a ﾞb", 2, vec!["a", "ﾞb"]),
            ("a ﾟb", 2, vec!["a", "ﾟb"]),
            ("ｶﾞﾞx", 3, vec!["ｶﾞﾞ", "x"]),
            ("界ﾞx", 3, vec!["界ﾞ", "x"]),
            ("ｶﾞﾞab", 2, vec!["ｶﾞﾞ", "ab"]),
            ("界ﾞab", 2, vec!["界ﾞ", "ab"]),
            ("abｶﾞﾞcd", 2, vec!["ab", "ｶﾞﾞ", "cd"]),
            ("ab界ﾞcd", 2, vec!["ab", "界ﾞ", "cd"]),
        ] {
            let ranges = wrap_ranges_trim(text, Options::new(width));
            let wrapped: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
            assert_eq!(wrapped, expected, "text={text:?}, width={width}");
        }

        for (text, emoji, sound_mark) in [("a👨‍👩 ﾞ", "👨‍👩", "ﾞ"), ("a👍🏻 ﾟ", "👍🏻", "ﾟ")]
        {
            for options in [
                Options::new(2),
                Options::new(2).word_separator(WordSeparator::AsciiSpace),
            ] {
                let ranges = wrap_ranges_trim(text, options);
                let wrapped: Vec<&str> = ranges.iter().map(|range| &text[range.clone()]).collect();
                assert_eq!(wrapped, ["a", emoji, sound_mark]);
            }
        }

        for grapheme in ["ｶﾞﾞ", "界ﾞ"] {
            for width in [1, 2] {
                let ranges = wrap_ranges(grapheme, Options::new(width));
                assert_eq!(
                    ranges,
                    std::iter::once(0..grapheme.len() + 1).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn wrap_ranges_preserve_crlf_source_boundaries_without_splitting_graphemes() {
        for prefix in ["ｶﾞ", "ﾊﾟ"] {
            let text = format!("{prefix}\r\nnext");
            for word_separator in [
                WordSeparator::UnicodeBreakProperties,
                WordSeparator::AsciiSpace,
            ] {
                for line_ending in [textwrap::LineEnding::LF, textwrap::LineEnding::CRLF] {
                    let options = Options::new(4)
                        .line_ending(line_ending)
                        .word_separator(word_separator)
                        .wrap_algorithm(textwrap::WrapAlgorithm::FirstFit);
                    let first_end =
                        prefix.len() + usize::from(line_ending == textwrap::LineEnding::LF);
                    let second_start = prefix.len() + "\r\n".len();
                    let ranges = wrap_ranges(&text, options.clone());
                    assert_eq!(ranges, [0..first_end + 1, second_start..text.len() + 1]);
                    for range in &ranges {
                        assert!(text.get(range.start..range.end - 1).is_some());
                    }
                    let trimmed = wrap_ranges_trim(&text, options);
                    assert_eq!(trimmed, [0..first_end, second_start..text.len()]);
                }
            }
        }
    }

    #[test]
    fn map_owned_wrapped_line_to_range_repro_overconsumes_repeated_prefix_patterns() {
        let text = "- - foo";
        let opts = Options::new(3)
            .initial_indent("- ")
            .subsequent_indent("- ")
            .word_separator(WordSeparator::AsciiSpace)
            .break_words(false);
        let wrapped = textwrap::wrap(text, opts);
        let Some(line) = wrapped.first() else {
            panic!("expected at least one wrapped line");
        };
        let mapped = map_owned_wrapped_line_to_range(text, 0, line.as_ref(), "- ");
        let expected_len = line
            .as_ref()
            .strip_prefix("- ")
            .unwrap_or(line.as_ref())
            .len();
        assert!(mapped.end.saturating_sub(mapped.start) <= expected_len);
    }

    #[test]
    fn wrap_ranges_recovers_with_non_space_indents() {
        let text = "The quick brown fox jumps over the lazy dog";
        let wrapped = textwrap::wrap(
            text,
            Options::new(12)
                .initial_indent("* ")
                .subsequent_indent("  "),
        );
        assert!(wrapped.iter().any(|line| matches!(line, Cow::Owned(_))));
        let ranges = wrap_ranges(
            text,
            Options::new(12)
                .initial_indent("* ")
                .subsequent_indent("  "),
        );
        assert!(!ranges.is_empty());
        let mut rebuilt = String::new();
        let mut cursor = 0usize;
        for range in ranges {
            let start = range.start.max(cursor).min(text.len());
            let end = range.end.min(text.len());
            if start < end {
                rebuilt.push_str(&text[start..end]);
            }
            cursor = cursor.max(end);
        }
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn wrap_ranges_trim_handles_owned_lines_with_penalty_char() {
        fn split_every_char(word: &str) -> Vec<usize> {
            word.char_indices().skip(1).map(|(idx, _)| idx).collect()
        }

        let text = "a_very_long_token_without_spaces";
        let opts = Options::new(8)
            .word_separator(WordSeparator::AsciiSpace)
            .word_splitter(WordSplitter::Custom(split_every_char))
            .break_words(false);
        let ranges = wrap_ranges_trim(text, opts);
        let rebuilt = ranges
            .iter()
            .map(|range| &text[range.clone()])
            .collect::<String>();
        assert_eq!(rebuilt, text);
        assert!(ranges.len() > 1);
    }
}
