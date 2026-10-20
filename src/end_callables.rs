//! Normalize the optional `do` separator of end-delimited free/module
//! callables before the ordinary keyword/indentation passes.
//!
//! The compiler treats this `do` as a semantics-free header/body boundary;
//! it is not a `... do / done` control-flow block. Compact same-line source
//! such as `fnc f() -> int do return 1; end` is expanded so the structural
//! formatter can own the final `end` normally. Strings and comments are
//! opaque.

use crate::{LexState, is_end_delimited_callable_header, scan_visible, visible_word_spans};

pub(crate) fn normalize(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut lex = LexState::default();

    for (line_index, line) in source.split('\n').enumerate() {
        if line_index != 0 {
            out.push('\n');
        }

        let initial = lex;
        scan_visible(line, &mut lex);
        let words = visible_word_spans(line, initial);
        let Some(do_span) = words.iter().find(|span| {
            &line[span.start..span.end] == "do"
                && is_end_delimited_callable_header(line[..span.end].trim())
        }) else {
            out.push_str(line);
            continue;
        };

        let has_visible_body = words.iter().any(|span| span.start >= do_span.end);
        if !has_visible_body {
            out.push_str(line);
            continue;
        }

        // The callable closer is the last lexically visible `end` word on a
        // compact line. Any earlier `end` remains part of the body (for
        // example a nested dispatch expression) and is handled by the normal
        // structural passes after this header/body split.
        let final_end = words
            .iter()
            .rev()
            .find(|span| span.start >= do_span.end && &line[span.start..span.end] == "end");

        out.push_str(line[..do_span.end].trim_end());
        out.push('\n');

        if let Some(end_span) = final_end {
            let body = line[do_span.end..end_span.start].trim();
            if !body.is_empty() {
                out.push_str(body);
                out.push('\n');
            }
            out.push_str(line[end_span.start..].trim_start());
        } else {
            out.push_str(line[do_span.end..].trim_start());
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn expands_compact_callable_do_body() {
        assert_eq!(
            normalize("fnc identity(int x) -> int do return x; end"),
            "fnc identity(int x) -> int do\nreturn x;\nend"
        );
    }

    #[test]
    fn preserves_multiline_header_only_do() {
        assert_eq!(
            normalize("fnc identity(int x) -> int do"),
            "fnc identity(int x) -> int do"
        );
    }

    #[test]
    fn higher_order_return_type_stays_in_header() {
        assert_eq!(
            normalize("fnc callback() -> () => void do return cb; end"),
            "fnc callback() -> () => void do\nreturn cb;\nend"
        );
    }

    #[test]
    fn quoted_and_commented_do_end_words_are_opaque() {
        let source = "fnc words() -> string do return \"do end\"; end // do end";
        assert_eq!(
            normalize(source),
            "fnc words() -> string do\nreturn \"do end\";\nend // do end"
        );
    }

    #[test]
    fn ordinary_do_block_is_untouched() {
        assert_eq!(normalize("while ready do work(); done"), "while ready do work(); done");
    }
}
