//! Text sanitisation for metadata fields: HTML stripping, entity decoding, whitespace normalisation.

/// Full sanitisation pipeline: decode entities -> strip HTML -> normalise whitespace.
pub fn sanitise(input: &str) -> String {
    let decoded = decode_entities(input);
    let stripped = strip_html(&decoded);
    normalise_whitespace(&stripped)
}

/// Strip HTML tags from a string. Simple state machine approach.
pub fn strip_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;

    // Strip CDATA wrappers. Note: if CDATA content contains a bare '<', it will
    // activate the tag-stripping state machine and may truncate subsequent text.
    // This is acceptable — OPF descriptions rarely contain raw '<' inside CDATA.
    let input = input.replace("<![CDATA[", "").replace("]]>", "");

    for c in input.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' && in_tag {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result
}

/// Normalise whitespace: collapse runs of whitespace to single space, trim.
pub fn normalise_whitespace(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut prev_ws = false;
    for c in input.chars() {
        if c.is_whitespace() {
            if !prev_ws && !result.is_empty() {
                result.push(' ');
            }
            prev_ws = true;
        } else {
            prev_ws = false;
            result.push(c);
        }
    }
    result.trim_end().to_string()
}

/// Decode common HTML entities and numeric character references.
pub fn decode_entities(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(amp_pos) = remaining.find('&') {
        result.push_str(&remaining[..amp_pos]);
        remaining = &remaining[amp_pos..];

        // Lookahead capped at 12 bytes: covers all named entities (max "&#x10FFFF;" = 10 chars)
        // and avoids scanning long runs of text for a missing semicolon.
        if let Some(semi_pos) = remaining[..remaining.len().min(12)].find(';') {
            let entity = &remaining[1..semi_pos];
            let decoded = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{00A0}'),
                _ if entity.starts_with('#') => parse_numeric_entity(&entity[1..]),
                _ => None,
            };
            if let Some(c) = decoded {
                result.push(c);
                remaining = &remaining[semi_pos + 1..];
            } else {
                result.push('&');
                remaining = &remaining[1..];
            }
        } else {
            result.push('&');
            remaining = &remaining[1..];
        }
    }
    result.push_str(remaining);
    result
}

fn parse_numeric_entity(s: &str) -> Option<char> {
    let num = if let Some(hex) = s.strip_prefix('x').or_else(|| s.strip_prefix('X')) {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        s.parse::<u32>().ok()?
    };
    char::from_u32(num)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_paragraph() {
        assert_eq!(strip_html("<p>Hello <em>world</em></p>"), "Hello world");
    }

    #[test]
    fn strip_html_br_variants() {
        assert_eq!(strip_html("line1<br>line2<br/>line3"), "line1line2line3");
    }

    #[test]
    fn strip_html_word_markup() {
        assert_eq!(strip_html("<o:p>text</o:p>"), "text");
    }

    #[test]
    fn strip_html_cdata() {
        assert_eq!(strip_html("<![CDATA[text]]>"), "text");
    }

    #[test]
    fn strip_html_empty() {
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn strip_html_no_tags() {
        assert_eq!(strip_html("plain text"), "plain text");
    }

    #[test]
    fn decode_named_entities() {
        assert_eq!(decode_entities("Smith &amp; Jones"), "Smith & Jones");
        assert_eq!(decode_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode_entities("&quot;hello&quot;"), "\"hello\"");
    }

    #[test]
    fn decode_numeric_entities() {
        assert_eq!(decode_entities("&#169;"), "\u{00A9}"); // copyright symbol
        assert_eq!(decode_entities("&#xA9;"), "\u{00A9}");
    }

    #[test]
    fn decode_unknown_entity_preserved() {
        assert_eq!(decode_entities("&unknown;"), "&unknown;");
    }

    #[test]
    fn normalise_whitespace_collapse() {
        assert_eq!(normalise_whitespace("  hello   world  "), "hello world");
    }

    #[test]
    fn normalise_whitespace_newlines() {
        assert_eq!(normalise_whitespace("line1\n  \n  line2"), "line1 line2");
    }

    #[test]
    fn sanitise_full_pipeline() {
        assert_eq!(sanitise("<p>Smith &amp; Jones</p>"), "Smith & Jones");
    }

    #[test]
    fn sanitise_nested_html() {
        assert_eq!(
            sanitise("<div><p>A <strong>bold</strong> statement.</p></div>"),
            "A bold statement."
        );
    }
}
