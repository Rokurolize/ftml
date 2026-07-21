use super::{ExtractedToken, Token};

pub fn extract_all(text: &str) -> Vec<ExtractedToken<'_>> {
    let mut tokens = Vec::with_capacity(text.len() / 2 + 2);
    push(&mut tokens, text, Token::InputStart, 0, 0);

    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        let (token, end) = next_token(text, bytes, start);
        push(&mut tokens, text, token, start, end);
        index = end;
    }

    push(&mut tokens, text, Token::InputEnd, text.len(), text.len());
    tokens
}

fn push<'t>(
    tokens: &mut Vec<ExtractedToken<'t>>,
    text: &'t str,
    token: Token,
    start: usize,
    end: usize,
) {
    tokens.push(ExtractedToken {
        token,
        slice: &text[start..end],
        span: start..end,
    });
}

fn next_token(text: &str, bytes: &[u8], start: usize) -> (Token, usize) {
    let byte = bytes[start];
    if byte.is_ascii_alphanumeric() {
        if matches!(byte, b'f' | b'h')
            && let Some(end) = scan_url(bytes, start)
        {
            return (Token::Url, end);
        }

        return scan_identifier_or_email(bytes, start);
    }

    if let Some(end) = scan_variable(bytes, start) {
        return (Token::Variable, end);
    }

    if let Some((token, end)) = scan_newline(bytes, start) {
        return (token, end);
    }

    if matches!(byte, b' ' | b'\t') {
        return (Token::Whitespace, scan_space(bytes, start));
    }

    if let Some((token, end)) = scan_literal(bytes, start) {
        return (token, end);
    }

    if let Some((token, end)) = scan_repeated_symbol(bytes, start) {
        return (token, end);
    }

    (Token::Other, next_char_end(text, start))
}

fn scan_literal(bytes: &[u8], start: usize) -> Option<(Token, usize)> {
    let result = match bytes[start] {
        b'@' if has(bytes, start, b"@@") => (Token::Raw, start + 2),
        b'@' if has(bytes, start, b"@<") => (Token::LeftRaw, start + 2),
        b'>' if has(bytes, start, b">@") => (Token::RightRaw, start + 2),
        b'[' if has(bytes, start, b"[!--") => (Token::LeftComment, start + 4),
        b'-' if has(bytes, start, b"--]") => (Token::RightComment, start + 3),
        b'[' if has(bytes, start, b"[[[[")
            && previous_byte(bytes, start) != Some(b'[') =>
        {
            (Token::LeftBracket, start + 1)
        }
        b']' if has(bytes, start, b"]]]]") => (Token::RightLink, start + 3),
        b'[' if has(bytes, start, b"[[[*") => (Token::LeftLinkStar, start + 4),
        b'[' if has(bytes, start, b"[[[") => (Token::LeftLink, start + 3),
        b'[' if has(bytes, start, b"[[$") => (Token::LeftMath, start + 3),
        b'[' if has(bytes, start, b"[[#") => (Token::LeftBlockAnchor, start + 3),
        b'[' if has(bytes, start, b"[[*") => (Token::LeftBlockStar, start + 3),
        b'[' if has(bytes, start, b"[[/") => (Token::LeftBlockEnd, start + 3),
        b'[' if has(bytes, start, b"[[") => (Token::LeftBlock, start + 2),
        b'[' if has(bytes, start, b"[#") => (Token::LeftBracketAnchor, start + 2),
        b'[' if has(bytes, start, b"[*") => (Token::LeftBracketStar, start + 2),
        b'[' => (Token::LeftBracket, start + 1),
        b'(' if has(bytes, start, b"((") => (Token::LeftParentheses, start + 2),
        b']' if has(bytes, start, b"]]]")
            && !is_right_link_trailing_bracket(bytes, start) =>
        {
            (Token::RightLink, start + 3)
        }
        b'$' if has(bytes, start, b"$]]") => (Token::RightMath, start + 3),
        b']' if has(bytes, start, b"]]")
            && !is_right_link_trailing_bracket(bytes, start) =>
        {
            (Token::RightBlock, start + 2)
        }
        b']' => (Token::RightBracket, start + 1),
        b')' if has(bytes, start, b"))") => (Token::RightParentheses, start + 2),
        b'*' if has(bytes, start, b"**") => (Token::Bold, start + 2),
        b'/' if has(bytes, start, b"//") => (Token::Italics, start + 2),
        b'_' if has(bytes, start, b"__") => (Token::Underline, start + 2),
        b'^' if has(bytes, start, b"^^") => (Token::Superscript, start + 2),
        b',' if has(bytes, start, b",,") => (Token::Subscript, start + 2),
        b'#' if has(bytes, start, b"##") => (Token::Color, start + 2),
        b'{' if has(bytes, start, b"{{") => (Token::LeftMonospace, start + 2),
        b'}' if has(bytes, start, b"}}") => (Token::RightMonospace, start + 2),
        b'|' if has(bytes, start, b"||~") => (Token::TableColumnTitle, start + 3),
        b'|' if has(bytes, start, b"||>") => (Token::TableColumnRight, start + 3),
        b'|' if has(bytes, start, b"||=") => (Token::TableColumnCenter, start + 3),
        b'|' if has(bytes, start, b"||") => (Token::TableColumn, start + 2),
        b'<' if has(bytes, start, b"<<") => (Token::LeftDoubleAngle, start + 2),
        b'|' => (Token::Pipe, start + 1),
        b'=' => (Token::Equals, start + 1),
        b':' => (Token::Colon, start + 1),
        b'_' => (Token::Underscore, start + 1),
        b'\\' if has(bytes, start, br#"\""#) => (Token::EscapedDoubleQuote, start + 2),
        b'"' => (Token::DoubleQuote, start + 1),
        b'\\' if has(bytes, start, br#"\\"#) => (Token::EscapedBackslash, start + 2),
        b'*' if bytes.get(start + 1) != Some(&b'*') => (Token::BulletItem, start + 1),
        b'#' if bytes.get(start + 1) != Some(&b'#') => (Token::NumberedItem, start + 1),
        _ => return None,
    };

    Some(result)
}

fn previous_byte(bytes: &[u8], start: usize) -> Option<u8> {
    start.checked_sub(1).map(|index| bytes[index])
}

fn is_right_link_trailing_bracket(bytes: &[u8], start: usize) -> bool {
    start >= 3
        && bytes[start - 3..start].iter().all(|&byte| byte == b']')
        && previous_byte(bytes, start - 3) != Some(b']')
}

fn scan_repeated_symbol(bytes: &[u8], start: usize) -> Option<(Token, usize)> {
    match bytes[start] {
        b'~' => {
            let end = scan_same(bytes, start, b'~');
            let count = end - start;
            if count >= 3 {
                if bytes.get(end) == Some(&b'<') {
                    Some((Token::ClearFloatLeft, end + 1))
                } else if bytes.get(end) == Some(&b'>') {
                    Some((Token::ClearFloatRight, end + 1))
                } else {
                    Some((Token::ClearFloatBoth, end))
                }
            } else if count == 2 {
                Some((Token::DoubleTilde, end))
            } else {
                None
            }
        }
        b'-' => {
            let end = scan_same(bytes, start, b'-');
            match end - start {
                2 => Some((Token::DoubleDash, end)),
                count if count >= 3 => Some((Token::TripleDash, end)),
                _ => None,
            }
        }
        b'>' => Some((Token::Quote, scan_same(bytes, start, b'>'))),
        b'+' => {
            let plus_end = scan_plus_heading(bytes, start);
            if plus_end > start {
                Some((Token::Heading, plus_end))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn scan_plus_heading(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && bytes[end] == b'+' && end - start < 6 {
        end += 1;
    }

    if end == start {
        return start;
    }

    if bytes.get(end) == Some(&b'*') && bytes.get(end + 1) != Some(&b'*') {
        end + 1
    } else {
        end
    }
}

fn scan_url(bytes: &[u8], start: usize) -> Option<usize> {
    let body_start = if has(bytes, start, b"http://") {
        start + 7
    } else if has(bytes, start, b"https://") {
        start + 8
    } else if has(bytes, start, b"ftp://") {
        start + 6
    } else {
        return None;
    };

    // Tokenization is context-free, so a reserved raw closer must remain a
    // separate token even after a URL scan has started. This intentionally
    // splits an ordinary `https://example.com/a>@b` at the `>@` marker.
    let mut end = body_start;
    while end < bytes.len()
        && !matches!(bytes[end], b'\n' | b'\r' | b' ' | b'"' | b'|' | b'[' | b']')
        && !has(bytes, end, b">@")
    {
        end += 1;
    }

    (end > body_start).then_some(end)
}

fn scan_identifier_or_email(bytes: &[u8], start: usize) -> (Token, usize) {
    debug_assert!(bytes[start].is_ascii_alphanumeric());

    let identifier_end = scan_identifier(bytes, start);
    match bytes.get(identifier_end) {
        Some(b' ' | b'\t' | b'\n' | b'\r') | None => {
            return (Token::Identifier, identifier_end);
        }
        _ => {}
    }

    // Angle brackets terminate an unquoted email address and also delimit raw
    // spans. Letting an email candidate cross either one can hide raw markers.
    let mut at = identifier_end;
    while at < bytes.len()
        && !matches!(
            bytes[at],
            b' ' | b'\t' | b'@' | b'[' | b']' | b'{' | b'}' | b'<' | b'>' | b'\n' | b'\r'
        )
    {
        at += 1;
    }
    if at == start || bytes.get(at) != Some(&b'@') {
        return (Token::Identifier, identifier_end);
    }

    let mut dot = at + 1;
    while dot < bytes.len()
        && !matches!(
            bytes[dot],
            b' ' | b'\t' | b'.' | b'[' | b']' | b'{' | b'}' | b'<' | b'>' | b'\n' | b'\r'
        )
    {
        dot += 1;
    }
    if dot == at + 1 || bytes.get(dot) != Some(&b'.') {
        return (Token::Identifier, identifier_end);
    }

    let mut end = dot + 1;
    while end < bytes.len()
        && !matches!(
            bytes[end],
            b' ' | b'\t' | b'[' | b']' | b'{' | b'}' | b'<' | b'>' | b'\n' | b'\r'
        )
    {
        end += 1;
    }

    if end > dot + 1 {
        (Token::Email, end)
    } else {
        (Token::Identifier, identifier_end)
    }
}

fn scan_identifier(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
        end += 1;
    }
    end
}

fn scan_variable(bytes: &[u8], start: usize) -> Option<usize> {
    if !has(bytes, start, b"{$") {
        return None;
    }

    let identifier_start = start + 2;
    let identifier_end = scan_identifier(bytes, identifier_start);
    if identifier_end == identifier_start || bytes.get(identifier_end) != Some(&b'}') {
        return None;
    }

    Some(identifier_end + 1)
}

fn scan_newline(bytes: &[u8], start: usize) -> Option<(Token, usize)> {
    let mut end = start;
    let mut count = 0;

    while let Some(next) = scan_newline_once(bytes, end) {
        end = next;
        count += 1;
    }

    match count {
        0 => None,
        1 => Some((Token::LineBreak, end)),
        _ => Some((Token::ParagraphBreak, end)),
    }
}

fn scan_newline_once(bytes: &[u8], start: usize) -> Option<usize> {
    match bytes.get(start) {
        Some(b'\r') if bytes.get(start + 1) == Some(&b'\n') => Some(start + 2),
        Some(b'\r' | b'\n') => Some(start + 1),
        _ => None,
    }
}

fn scan_space(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
        end += 1;
    }
    end
}

fn scan_same(bytes: &[u8], start: usize, byte: u8) -> usize {
    let mut end = start;
    while end < bytes.len() && bytes[end] == byte {
        end += 1;
    }
    end
}

fn has(bytes: &[u8], start: usize, literal: &[u8]) -> bool {
    bytes
        .get(start..start.saturating_add(literal.len()))
        .is_some_and(|candidate| candidate == literal)
}

fn next_char_end(text: &str, start: usize) -> usize {
    if text.as_bytes()[start].is_ascii() {
        start + 1
    } else {
        start
            + text[start..]
                .chars()
                .next()
                .expect("valid UTF-8")
                .len_utf8()
    }
}
