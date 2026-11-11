//! Narrow, source-aware AttoLang `!eq` -> `neq` spelling normalization.
//!
//! This does not reinterpret Erlang `==` / `!=` tokens. It only canonicalizes
//! AttoLang's already-defined `!eq` inequality alias in source-code regions.
//! Nested comments, strings, single quotes, backtick literals and UTF-8 bytes
//! remain untouched. The semantic checker, not the formatter, decides type
//! compatibility, evaluation order, and full-AOT eligibility.

pub(crate) fn canonicalize_neq_alias(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut result = String::with_capacity(source.len());
    let mut index = 0;
    let mut last_copied = 0;
    let mut block_depth = 0usize;
    let mut quote: Option<u8> = None;
    let mut escape = false;

    while index < bytes.len() {
        let ch = bytes[index];
        let next = bytes.get(index + 1).copied();

        if block_depth != 0 {
            if ch == b'/' && next == Some(b'*') {
                block_depth += 1;
                index += 2;
                continue;
            }
            if ch == b'*' && next == Some(b'/') {
                block_depth -= 1;
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(delimiter) = quote {
            if escape {
                escape = false;
            } else if ch == b'\\' {
                escape = true;
            } else if ch == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }

        if ch == b'/' && next == Some(b'/') {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if ch == b'/' && next == Some(b'*') {
            block_depth = 1;
            index += 2;
            continue;
        }
        if matches!(ch, b'"' | b'\'' | b'`') {
            quote = Some(ch);
            index += 1;
            continue;
        }

        if ch == b'!' && next == Some(b'e') && bytes.get(index + 2) == Some(&b'q') {
            // Never turn a prefix of an identifier into an operator.
            let suffix = &source[index + 3..];
            if suffix.chars().next().is_some_and(is_name_character) {
                index += 3;
                continue;
            }
            result.push_str(&source[last_copied..index]);
            // An adjacent left operand must not concatenate with the
            // canonical `neq` identifier: `foo!eq bar` -> `foo neq bar`.
            if source[..index].chars().next_back().is_some_and(is_name_character) {
                result.push(' ');
            }
            result.push_str("neq");
            index += 3;
            last_copied = index;
            continue;
        }
        index += 1;
    }

    result.push_str(&source[last_copied..]);
    result
}

fn is_name_character(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::canonicalize_neq_alias;

    #[test]
    fn preserves_nested_comments_and_unicode() {
        let input = "α !eq β /* outer !eq /* inner !eq */ !eq */ // !eq\n\"!eq\" !eq `!eq`";
        let expected = "α neq β /* outer !eq /* inner !eq */ !eq */ // !eq\n\"!eq\" neq `!eq`";
        assert_eq!(canonicalize_neq_alias(input), expected);
    }

    #[test]
    fn no_rewrite_inside_identifiers_or_in_legacy_not_equal() {
        let input = "foo!equivalent a != b a!eq b";
        assert_eq!(canonicalize_neq_alias(input), "foo!equivalent a != b a neq b");
    }
}
