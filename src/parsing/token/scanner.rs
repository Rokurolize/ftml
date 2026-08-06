use super::{ExtractedToken, Token};

pub fn extract_all(text: &str) -> Vec<ExtractedToken<'_>> {
    extract_all_with_trailing_comment_closer(text, false)
}

pub(super) fn extract_all_with_trailing_comment_closer(
    text: &str,
    trailing_comment_closer: bool,
) -> Vec<ExtractedToken<'_>> {
    extract_all_with_comment_scan_visits(text, trailing_comment_closer).0
}

fn extract_all_with_comment_scan_visits(
    text: &str,
    trailing_comment_closer: bool,
) -> (Vec<ExtractedToken<'_>>, usize) {
    let mut tokens = Vec::with_capacity(text.len() / 2 + 2);
    push(&mut tokens, text, Token::InputStart, 0, 0);

    let bytes = text.as_bytes();
    let mut comment_closers = CommentCloserCursor::new(bytes);
    // A failed `[!--` candidate must retain ordinary token ownership. Once a
    // validated opener is active, remember the exact closer bracket so a
    // following `]]` run cannot swallow it into a block or link token.
    let mut comment_active = false;
    let mut comment_closer_bracket = None;
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        let force_right_bracket = comment_closer_bracket == Some(start);
        if force_right_bracket {
            comment_closer_bracket = None;
        }
        if comment_active && let Some(bracket) = comment_closer_bracket_at(bytes, start) {
            comment_active = false;
            comment_closer_bracket = Some(bracket);
        }
        let valid_comment_opener = has(bytes, start, b"[!--")
            && (trailing_comment_closer
                || comment_closers.has_at_or_after(start + b"[!--".len()));
        let was_comment_active = comment_active;
        let (token, end) = next_token(
            text,
            bytes,
            start,
            valid_comment_opener,
            force_right_bracket,
        );
        if was_comment_active
            && text[start..end].ends_with("--")
            && bytes.get(end) == Some(&b']')
        {
            comment_active = false;
            comment_closer_bracket = Some(end);
        }
        if token == Token::LeftComment {
            comment_active = true;
        }
        push(&mut tokens, text, token, start, end);
        index = end;
    }

    push(&mut tokens, text, Token::InputEnd, text.len(), text.len());
    (tokens, comment_closers.scan_visits)
}

struct CommentCloserCursor<'a> {
    bytes: &'a [u8],
    initialized: bool,
    next: Option<usize>,
    search_start: usize,
    scan_visits: usize,
}

impl<'a> CommentCloserCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            initialized: false,
            next: None,
            search_start: 0,
            scan_visits: 0,
        }
    }

    fn has_at_or_after(&mut self, minimum: usize) -> bool {
        if !self.initialized {
            self.initialized = true;
            self.find_next();
        }
        while self.next.is_some_and(|position| position < minimum) {
            self.find_next();
        }
        self.next.is_some()
    }

    fn find_next(&mut self) {
        // search_start only moves forward, so all opener checks together
        // visit each possible closer position at most once.
        while self.search_start + 2 < self.bytes.len() {
            let position = self.search_start;
            self.search_start += 1;
            self.scan_visits += 1;
            if has(self.bytes, position, b"--]") {
                self.next = Some(position);
                return;
            }
        }
        self.search_start = self.bytes.len();
        self.next = None;
    }
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

fn next_token(
    text: &str,
    bytes: &[u8],
    start: usize,
    valid_comment_opener: bool,
    force_right_bracket: bool,
) -> (Token, usize) {
    let byte = bytes[start];
    if is_discarded_control(byte) {
        return (Token::DiscardedControl, start + 1);
    }
    if byte.is_ascii_alphanumeric() {
        if matches!(byte, b'f' | b'h' | b'm')
            && let Some(end) = scan_url(bytes, start)
        {
            return (Token::Url, end);
        }

        return scan_identifier_or_email(bytes, start);
    }

    if let Some(end) = scan_variable(bytes, start) {
        return (Token::Variable, end);
    }

    if byte == b'\\' && bytes.get(start + 1) == Some(&b'\n') {
        return (Token::LineBreak, start + 2);
    }

    if let Some((token, end)) = scan_newline(bytes, start) {
        return (token, end);
    }

    if matches!(byte, b' ' | b'\t') {
        return (Token::Whitespace, scan_space(bytes, start));
    }

    if let Some((token, end)) =
        scan_literal(bytes, start, valid_comment_opener, force_right_bracket)
    {
        return (token, end);
    }

    if let Some((token, end)) = scan_repeated_symbol(bytes, start) {
        return (token, end);
    }

    (Token::Other, next_char_end(text, start))
}

fn is_discarded_control(byte: u8) -> bool {
    matches!(byte, 0x00..=0x08 | 0x0b..=0x0c | 0x0e..=0x1a | 0x1c..=0x1f)
}

fn scan_literal(
    bytes: &[u8],
    start: usize,
    valid_comment_opener: bool,
    force_right_bracket: bool,
) -> Option<(Token, usize)> {
    let result = match bytes[start] {
        b'@' if has(bytes, start, b"@@") => (Token::Raw, start + 2),
        b'@' if has(bytes, start, b"@<") => (Token::LeftRaw, start + 2),
        b'>' if has(bytes, start, b">@") => (Token::RightRaw, start + 2),
        b'[' if valid_comment_opener => (Token::LeftComment, start + 4),
        b'[' if has(bytes, start, b"[[[[")
            && repeated_symbol_offset(bytes, start, b'[').is_multiple_of(4) =>
        {
            (Token::LeftBracket, start + 1)
        }
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
        b']' if force_right_bracket => (Token::RightBracket, start + 1),
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
        b'\\' if start + 1 == bytes.len() => (Token::LineBreak, start + 1),
        b'*' if bytes.get(start + 1) != Some(&b'*') => (Token::BulletItem, start + 1),
        b'#' if bytes.get(start + 1) != Some(&b'#') => (Token::NumberedItem, start + 1),
        _ => return None,
    };

    Some(result)
}

fn repeated_symbol_offset(bytes: &[u8], start: usize, symbol: u8) -> usize {
    let mut run_start = start;
    while run_start > 0 && bytes[run_start - 1] == symbol {
        run_start -= 1;
    }
    start - run_start
}

fn is_right_link_trailing_bracket(bytes: &[u8], start: usize) -> bool {
    repeated_symbol_offset(bytes, start, b']') % 4 == 3
}

fn scan_repeated_symbol(bytes: &[u8], start: usize) -> Option<(Token, usize)> {
    match bytes[start] {
        b'~' => {
            let end = scan_same(bytes, start, b'~');
            let count = end - start;
            if count >= 4 {
                if bytes.get(end) == Some(&b'<') {
                    Some((Token::ClearFloatLeft, end + 1))
                } else if bytes.get(end) == Some(&b'>') {
                    Some((Token::ClearFloatRight, end + 1))
                } else {
                    Some((Token::ClearFloatBoth, end))
                }
            } else if count >= 2 {
                Some((Token::DoubleTilde, start + 2))
            } else {
                None
            }
        }
        b'-' => {
            let end = scan_same(bytes, start, b'-');
            match end - start {
                2 => Some((Token::DoubleDash, end)),
                3 => Some((Token::DoubleDash, start + 2)),
                count if count >= 4 => Some((Token::TripleDash, end)),
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
    } else if has(bytes, start, b"mailto:") {
        start + 7
    } else {
        return None;
    };

    // Tokenization is context-free, so a reserved raw closer must remain a
    // separate token even after a URL scan has started. This intentionally
    // splits an ordinary `https://example.com/a>@b` at the `>@` marker.
    let mut end = body_start;
    while end < bytes.len()
        && !matches!(
            bytes[end],
            b'\n' | b'\r' | b' ' | b'\t' | b'"' | b'\'' | b'|' | b'[' | b']'
        )
        && !is_discarded_control(bytes[end])
        && !has(bytes, end, b">@")
        && !has(bytes, end, b"@@")
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
            b' ' | b'\t'
                | b'%'
                | b'@'
                | b'['
                | b']'
                | b'{'
                | b'}'
                | b'<'
                | b'>'
                | b'|'
                | b'('
                | b')'
                | b'"'
                | b':'
                | b'\n'
                | b'\r'
        )
        && !is_discarded_control(bytes[at])
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
            b' ' | b'\t'
                | b'.'
                | b'@'
                | b'['
                | b']'
                | b'{'
                | b'}'
                | b'<'
                | b'>'
                | b'\n'
                | b'\r'
        )
        && !is_discarded_control(bytes[dot])
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
            b' ' | b'\t' | b'@' | b'[' | b']' | b'{' | b'}' | b'<' | b'>' | b'\n' | b'\r'
        )
        && !is_discarded_control(bytes[end])
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

fn comment_closer_bracket_at(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'-') {
        return None;
    }
    let end = scan_same(bytes, start, b'-');
    (end - start >= 2 && bytes.get(end) == Some(&b']')).then_some(end)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_continued_boundary_is_one_line_break_token() {
        let tokens = extract_all("\\\n");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token, Token::InputStart);
        assert_eq!(tokens[1].token, Token::LineBreak);
        assert_eq!(tokens[1].slice, "\\\n");
        assert_eq!(tokens[2].token, Token::InputEnd);
    }

    #[test]
    fn malformed_comment_candidate_scan_is_linear_and_token_memory_is_bounded() {
        let input = "A[!--hidden------ ]B".repeat(32_768);
        let (tokens, comment_scan_visits) =
            extract_all_with_comment_scan_visits(&input, false);

        assert!(comment_scan_visits <= input.len());
        assert!(tokens.len() <= input.len() + 2);
        assert!(tokens.iter().all(|token| token.token != Token::LeftComment),);
    }

    #[test]
    fn comment_closer_scan_is_lazy_for_inputs_without_candidates() {
        let input = "ordinary --] punctuation".repeat(4_096);
        let (_, comment_scan_visits) =
            extract_all_with_comment_scan_visits(&input, false);

        assert_eq!(comment_scan_visits, 0);
    }

    #[test]
    fn active_comment_closer_owns_only_the_first_bracket_of_a_run() {
        let active = extract_all("[!--x--]]]");
        let active_tokens = active.iter().map(|token| token.token).collect::<Vec<_>>();
        assert_eq!(
            active_tokens,
            vec![
                Token::InputStart,
                Token::LeftComment,
                Token::Identifier,
                Token::DoubleDash,
                Token::RightBracket,
                Token::RightBlock,
                Token::InputEnd,
            ],
        );

        let ordinary = extract_all("x--]]]");
        assert!(ordinary.iter().any(|token| token.token == Token::RightLink),);
    }
}
