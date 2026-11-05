//! Input generators: corpus mutation, splicing, and adversarial synthesis.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }

    pub fn chance(&mut self, num: usize, den: usize) -> bool {
        self.below(den) < num
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// Mostly small sizes, occasionally large.
    pub fn size(&mut self) -> usize {
        match self.below(10) {
            0..=5 => 1 + self.below(8),
            6..=7 => 8 + self.below(64),
            8 => 64 + self.below(1024),
            _ => 1024 + self.below(1 << 16),
        }
    }
}

pub const BLOCKS: &[&str] = &[
    "a", "anchor", "anchortarget", "audio", "b", "bibcite", "bibliography", "block",
    "blockquote", "bold", "button", "cell", "char", "character", "checkbox", "code",
    "collapsible", "date", "del", "deletion", "div", "div_", "em", "embed", "embedvideo",
    "emphasis", "eqref", "equation", "eref", "f<image", "f<toc", "f=image", "f>image",
    "f>toc", "<image", "=image", ">image", "file", "footnote", "footnoteblock", "gallery",
    "hcell", "hidden", "highlight", "html", "i", "ifcategory", "iframe", "iftags", "image",
    "include", "include-elements", "ins", "insertion", "invisible", "italics", "later",
    "li", "lines", "mark", "math", "module", "mono", "monospace", "newlines", "note", "ol",
    "p", "paragraph", "quote", "radio", "radio-button", "raw", "rb", "row", "rt", "ruby",
    "ruby2", "rubytext", "s", "size", "social", "span", "span_", "strikethrough", "strong",
    "sub", "subscript", "sup", "super", "superscript", "tab", "table", "tabs", "tabview",
    "target", "toc", "tt", "u", "ul", "underline", "user", "video", "$", "#if", "#ifexpr",
    "#expr", "#ifeq", "*user", "*div", "DIV", "Div", "COLLAPSIBLE", "İnclude", "module654",
    "=", "<", ">", "==", "f", "*", "/div", "/span", "/collapsible", "/tabview", "/tab",
    "/table", "/row", "/cell", "/footnote", "/math", "/code", "/html", "/iftags", "/ul",
    "/li", "/ol", "/size", "/note", "/hidden", "/a", "/embed", "/iframe", "/gallery",
];

pub const MODULES: &[&str] = &[
    "Backlinks", "CSS", "Categories", "CustomModule", "Join", "ListPages", "PageTree",
    "Rate", "Redirect", "TagCloud", "ListUsers", "css", "listpages", "Unknown", "",
];

pub const ARGS: &[&str] = &[
    "", " ", " x", " class=\"x\"", " style=\"color: red\"", " id=\"\"", " id=\"u-",
    " width=\"99999999999999999999\"", " height=\"-1\"", " size=\"0\"", " size=\"-5\"",
    " format=\"%Y-%m-%d %H:%M:%S\"", " format=\"%\"", " format=\"%%%\"", " format=\"%Q\"",
    " type=\"\"", " type=\"html\"", " show=\"+ show\" hide=\"- hide\"", " folded=\"no\"",
    " title=\"\u{FFFD}\"", " =", " = ", " \"", " '", " a=\"b", " a=", " a=b c=d", " |",
    " | x", " 1", " 0", " -1", " 99999999999999999999999", " 1e309", " NaN", " inf",
    " 1.5", " 100%", " 100px", " 0x10", " #FF0000", " #", " #xD800", " #55296",
    " #1114112", " &#xD800;", " \u{0}", " \r", " \n", " \t", " İ", " Ⱥ", " ﬃ", " 🦀",
    " +a -b", " +", " -", " _", " +_", " -_", " a b c d e f g", " 999999999999",
    " -999999999999", " 253402300800", " -62167219201", " 9223372036854775807",
    " -9223372036854775808", " 18446744073709551616", " link=\"https://x\"",
    " link=\"*\"", " link=\"#\"", " src=\"\"", " src=\"x\"", " limit=\"0\"",
    " perPage=\"-1\"", " separate=\"no\"", " name=\"\"", " name=\"x\"", " role=\"x\"",
    " checked", " hidden", " show-hide=\"x\"", " Ⱦ", " ı", " ẞ", " ǅ",
];

pub const TOKENS: &[&str] = &[
    "[[", "]]", "[[/", "[[*", "[[[", "]]]", "[", "]", "@@", "@<", ">@", "{{", "}}", "**",
    "//", "__", "--", ",,", "^^", "##", "||", "||~", "||<", "||>", "||=", "~~~~", "~~~",
    "~~", "\n", "\r", "\r\n", "\n\n", "\r\r\n", "\0", "\t", " ", "  ", ">", "> ", ">>",
    "* ", "# ", "+ ", "++ ", "= ", ":", ": ", "|", "\"", "'", "=", "%%", "[!--", "--]",
    "{$", "{$x}", "%%content%%", "[[#if", "[[#ifexpr", "[[#expr", "_", "\\", "&", "&#",
    "&#x", ";", "&#xD800;", "&#55296;", "&#xDFFF;", "&#x10FFFF;", "&#x110000;", "&#0;",
    "&#00000000000000000000065;", "&#99999999999;", "&amp;", "&nbsp;", "&#x;", "&#;",
    "\\uD800", "\\uDFFF", "\\ud83d\\ude00", "\\u{D800}", "\\x00", "%ED%A0%80",
    "%F0%9F", "%", "%%%", "%0", "http://", "https://", "mailto:", "javascript:", "#",
    "#toc", "*", "------", "-----", "----", "+++", "=====", "[[toc]]", "[[footnote]]",
    "[[/footnote]]", "[[div]]", "[[/div]]", "[[span]]", "[[/span]]", "[[collapsible]]",
    "[[/collapsible]]", "[[tabview]]", "[[tab x]]", "[[/tab]]", "[[/tabview]]",
    "[[table]]", "[[row]]", "[[cell]]", "[[/cell]]", "[[/row]]", "[[/table]]",
    "[[code]]", "[[/code]]", "[[math]]", "[[/math]]", "[[$", "$]]", "[[html]]",
    "[[/html]]", "[[iftags +a]]", "[[/iftags]]", "[[include x]]", "[[module ListPages]]",
    "[[/module]]", "[[footnoteblock]]", "[[bibliography]]", "[[/bibliography]]",
    "[[bibcite x]]", "[[eref x]]", "[[char #xD800]]", "[[date 0]]", "[[user x]]",
    "[[*user x]]", "[[image x.png]]", "[[gallery]]", "[[/gallery]]", "[[ul]]", "[[li]]",
    "[[/li]]", "[[/ul]]", "[[ruby]]", "[[rt]]", "[[/rt]]", "[[/ruby]]", "[[size 200%]]",
    "[[/size]]", "[[note]]", "[[/note]]", "[[raw]]", "[[/raw]]", "@@@@", "@@@@@@",
    "[[[x]]]", "[[[x|y]]]", "[[[*x]]]", "[http://x y]", "[#x y]", "[*http://x y]",
    "é", "漢", "🦀", "\u{301}", "\u{200D}", "\u{FEFF}", "\u{2028}", "\u{2029}", "\u{85}",
    "\u{A0}", "\u{3000}", "\u{FFFD}", "\u{10FFFF}", "\u{E000}", "Ǆ", "ǅ", "ß", "İ", "ẞ",
    "ﬃ", "ﬁ", "Σ", "ς", "\u{212A}", "\u{212B}", "ı", "Ⱥ", "Ⱦ", "ɐ", "ꞵ", "①", "㌀",
    "\u{1F1EF}\u{1F1F5}", "\u{0}\u{0}", "\u{1}", "\u{7F}", "\u{8}", "\u{B}", "\u{C}",
    "\u{1B}", "\u{2066}", "\u{202E}", "\u{FE0F}", "\u{E0001}", "\u{110BD}", "\u{1D11E}",
    "+", "-", "!", "?", ".", ",", "...", "``", "''", "<<", ">>", "<", "<<<", ">>>",
    "~", "^", "@", "$", "{", "}", "(", ")", "[[=", "[[<", "[[>", "[[==]]", "[[/=]]",
    "[[f>toc]]", "[[f<image x]]", "[[>]]", "[[/>]]", "[[<]]", "[[/<]]", "[[==]]",
    "--]", "[!--x--]", "@@x@@", "{{x}}", "**x**", "//x//", "__x__", "--x--", ",,x,,",
    "^^x^^", "##red|x##", "##", "##|", "#|", "##red|", "[[span style=\"",
];

pub const EOLS: &[&str] = &["\n", "\r\n", "\r", "\n\n", "\r\n\r\n", "\n\r", " \n", "\t\n"];

/// Characters whose case mapping or normalization changes the byte length.
pub const SHIFTY: &[&str] = &[
    "İ", "ẞ", "ﬃ", "ﬁ", "Ⱥ", "Ⱦ", "ɐ", "ı", "\u{212A}", "\u{212B}", "Σ", "ǅ", "①",
    "㌀", "\u{FB00}", "ŉ", "ΐ", "ᾳ", "\u{1E9E}", "ꞵ", "\u{10400}", "\u{1E921}",
];

pub fn rand_char(rng: &mut Rng) -> char {
    match rng.below(8) {
        0 => rng.below(0x20) as u8 as char,
        1..=3 => (0x20 + rng.below(0x5F)) as u8 as char,
        4 => char::from_u32(0x80 + rng.below(0x780) as u32).unwrap_or('x'),
        5 => char::from_u32(0x800 + rng.below(0xD000) as u32).unwrap_or('x'),
        6 => char::from_u32(0x10000 + rng.below(0x100000) as u32).unwrap_or('x'),
        _ => *rng.pick(&['[', ']', '*', '/', '_', '-', '@', '{', '}', '|', '>', '#', '+', '=', '"', '\n', '\r', ' ', '\0', '%', '&', ';', ':', '$', '!', '~', '^', ',']),
    }
}

fn floor_boundary(s: &str, mut i: usize) -> usize {
    if i > s.len() {
        return s.len();
    }
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn rand_pos(rng: &mut Rng, s: &str) -> usize {
    floor_boundary(s, rng.below(s.len() + 1))
}

fn rand_range(rng: &mut Rng, s: &str) -> (usize, usize) {
    let a = rand_pos(rng, s);
    let b = rand_pos(rng, s);
    if a <= b { (a, b) } else { (b, a) }
}

fn rand_fragment(rng: &mut Rng) -> String {
    match rng.below(10) {
        0..=5 => rng.pick(TOKENS).to_string(),
        6 => rng.pick(SHIFTY).to_string(),
        7 => rng.pick(EOLS).to_string(),
        8 => format!("[[{}{}]]", rng.pick(BLOCKS), rng.pick(ARGS)),
        _ => {
            let n = 1 + rng.below(4);
            (0..n).map(|_| rand_char(rng)).collect()
        }
    }
}

pub fn mutate(rng: &mut Rng, input: &mut String, corpus: &[String]) {
    let rounds = 1 + rng.below(if rng.chance(1, 4) { 32 } else { 6 });
    for _ in 0..rounds {
        if input.len() > 1 << 18 {
            let cut = floor_boundary(input, 1 << 17);
            input.truncate(cut);
        }
        match rng.below(16) {
            0 | 1 => {
                let p = rand_pos(rng, input);
                let f = rand_fragment(rng);
                input.insert_str(p, &f);
            }
            2 => {
                let (a, b) = rand_range(rng, input);
                let b = floor_boundary(input, a + (b - a).min(1 + rng.below(16)));
                input.replace_range(a..b, "");
            }
            3 => {
                let (a, b) = rand_range(rng, input);
                let b = floor_boundary(input, a + (b - a).min(1 + rng.below(8)));
                let f = rand_fragment(rng);
                input.replace_range(a..b, &f);
            }
            4 => {
                // Duplicate a range (builds nesting / long runs).
                let (a, b) = rand_range(rng, input);
                let chunk = input[a..b].to_string();
                let times = 1 + rng.below(if rng.chance(1, 8) { 200 } else { 3 });
                let p = rand_pos(rng, input);
                input.insert_str(p, &chunk.repeat(times));
            }
            5 => {
                // Truncate: leaves constructs unterminated.
                let p = rand_pos(rng, input);
                input.truncate(p);
            }
            6 => {
                // Splice with another corpus entry.
                let other = rng.pick(corpus);
                let (a, b) = rand_range(rng, other);
                let p = rand_pos(rng, input);
                input.insert_str(p, &other[a..b]);
            }
            7 => {
                // Byte-level mutation, re-decoded lossily.
                let mut bytes = std::mem::take(input).into_bytes();
                let n = 1 + rng.below(4);
                for _ in 0..n {
                    if bytes.is_empty() {
                        bytes.push(rng.next() as u8);
                        continue;
                    }
                    let i = rng.below(bytes.len());
                    match rng.below(4) {
                        0 => bytes[i] ^= 1 << rng.below(8),
                        1 => bytes[i] = rng.next() as u8,
                        2 => bytes.insert(i, rng.next() as u8),
                        _ => {
                            bytes.remove(i);
                        }
                    }
                }
                *input = match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                };
            }
            8 => {
                // Line-ending rewrite.
                let from = *rng.pick(&["\n", "\r\n", "\r"]);
                let to = *rng.pick(EOLS);
                if rng.chance(1, 2) {
                    *input = input.replace(from, to);
                } else {
                    *input = input.replacen(from, to, 1 + rng.below(4));
                }
            }
            9 => {
                // Long token run.
                let p = rand_pos(rng, input);
                let unit = rand_fragment(rng);
                let n = rng.size();
                input.insert_str(p, &unit.repeat(n.min(200_000 / unit.len().max(1))));
            }
            10 => {
                // Wrap a range in a block.
                let (a, b) = rand_range(rng, input);
                let name = *rng.pick(BLOCKS);
                let args = *rng.pick(ARGS);
                let eol = *rng.pick(EOLS);
                let close = if rng.chance(3, 4) { format!("{eol}[[/{name}]]") } else { String::new() };
                input.insert_str(b, &close);
                input.insert_str(a, &format!("[[{name}{args}]]{eol}"));
            }
            11 => {
                // Swap case of ASCII / shifty characters.
                let (a, b) = rand_range(rng, input);
                let seg: String = if rng.chance(1, 2) { input[a..b].to_uppercase() } else { input[a..b].to_lowercase() };
                input.replace_range(a..b, &seg);
            }
            12 => {
                let p = rand_pos(rng, input);
                input.insert(p, rand_char(rng));
            }
            13 => {
                // Insert shifty char next to a syntax char.
                if let Some(i) = input.find(|c: char| "[]*/_-@{}|>#+=\"".contains(c)) {
                    let i = if rng.chance(1, 2) { i } else { rand_pos(rng, input) };
                    input.insert_str(i, rng.pick(SHIFTY));
                }
            }
            14 => {
                // Replace all of one char with another.
                if !input.is_empty() {
                    let p = rand_pos(rng, input);
                    if let Some(c) = input[p..].chars().next() {
                        let r = rand_fragment(rng);
                        *input = input.replacen(c, &r, 1 + rng.below(8));
                    }
                }
            }
            _ => {
                let s = synth(rng);
                let p = rand_pos(rng, input);
                input.insert_str(p, &s);
            }
        }
    }
}

pub fn synth(rng: &mut Rng) -> String {
    let eol = *rng.pick(EOLS);
    let name = *rng.pick(BLOCKS);
    let args = *rng.pick(ARGS);
    let depth = match rng.below(6) {
        0 => 1 + rng.below(3),
        1..=3 => 1 + rng.below(40),
        4 => 90 + rng.below(30),
        _ => 1000 + rng.below(3000),
    };
    let text = rand_fragment(rng);
    match rng.below(26) {
        0 => {
            // Nested block, maybe partially closed.
            let close = depth - rng.below(depth + 1);
            let mut s = format!("[[{name}{args}]]{eol}").repeat(depth);
            s.push_str(&text);
            s.push_str(&format!("{eol}[[/{name}]]").repeat(close));
            s
        }
        1 => format!("{} {text}{eol}after", ">".repeat(depth)),
        2 => format!("{}* {text}{eol}{}# x", " ".repeat(depth), " ".repeat(depth / 2)),
        3 => {
            let units = ["**", "//", "__", "--", ",,", "^^", "{{", "@@", "##red|", "[[span]]", "[[[", "[!--", "@<"];
            let mut s = String::new();
            for _ in 0..depth.min(4000) {
                s.push_str(rng.pick(&units));
                s.push_str(rng.pick(&["a", " ", "", "é", "\n"]));
            }
            s
        }
        4 => {
            let unit = *rng.pick(&["[", "]", "*", "-", "=", "+", "@", "{", "|", "~", "#", "a", "_", ",", "^", "/", ":", "\"", ">", "<", "\\", "&", "%", "$", "!", "\0", "\r", "\n", "\r\n", " ", "é", "🦀", "\u{301}", "[[", "]]"]);
            let n = rng.size().max(depth);
            unit.repeat(n.min(200_000))
        }
        5 => format!("[[{name}{args}"),
        6 => format!("[[{name}{args}]]{eol}{text}"),
        7 => format!("{text}[[/{name}]]"),
        8 => format!("[[{name}{args}]]{eol}{text}{eol}[[/{}]]", rng.pick(BLOCKS)),
        9 => {
            let opener = *rng.pick(&["**", "//", "__", "--", ",,", "^^", "{{", "@@", "@<", "[[[", "[", "[!--", "##", "##red|", "[http://x", "[[#if", "[[#expr", "[[#ifexpr", "{$", "%%", "||", "||~", "[[$", "[[footnote]]", "~~~~", "[*", "[#", "[[[*"]);
            format!("{opener}{text}{}", if rng.chance(1, 3) { eol } else { "" })
        }
        10 => {
            let esc = *rng.pick(&["&#xD800;", "&#xDBFF;&#xDC00;", "&#xDFFF;", "&#55296;", "&#57343;", "&#x110000;", "&#1114112;", "&#99999999999999999999;", "&#0;", "&#x0;", "&#;", "&#x;", "\\uD800", "\\uDC00\\uD800", "\\u{D800}", "%ED%B0%80", "&#xFFFE;", "&#xFFFF;", "&#x1F600;", "&#128512;"]);
            match rng.below(8) {
                0 => format!("[[char {}]]", esc.trim_start_matches('&').trim_end_matches(';')),
                1 => format!("@<{esc}>@"),
                2 => format!("[[[{esc}|{esc}]]]"),
                3 => format!("[[span style=\"{esc}\"]]{esc}[[/span]]"),
                4 => format!("[[#if {esc} | {esc} | {esc}]]"),
                5 => format!("[[a href=\"{esc}\"]]x[[/a]]"),
                6 => format!("[[math]]{eol}\\text{{{esc}}}{eol}[[/math]]"),
                _ => format!("{esc}{text}{esc}"),
            }
        }
        11 => {
            let m = *rng.pick(MODULES);
            format!("[[module {m}{args}]]{eol}{text}{eol}[[/module]]")
        }
        12 => {
            let mut s = String::new();
            for i in 0..depth.min(300) {
                s.push_str(&format!("[[tabview]]{eol}[[tab t{i}]]{eol}"));
            }
            s.push_str(&text);
            s
        }
        13 => {
            let mut s = String::new();
            for _ in 0..depth.min(500) {
                s.push_str(&format!("[[collapsible show=\"+{text}\"{args}]]{eol}> "));
            }
            s
        }
        14 => {
            let cells = 1 + rng.below(20);
            let rows = 1 + rng.below(20);
            let mut s = String::new();
            for _ in 0..rows {
                for _ in 0..cells {
                    s.push_str(rng.pick(&["||", "||~ ", "||< ", "||> ", "||= ", "|||| ", "||||||"]));
                    s.push_str(&text);
                }
                if rng.chance(3, 4) { s.push_str("||"); }
                s.push_str(eol);
            }
            s
        }
        15 => {
            let f = *rng.pick(&["if", "ifexpr", "expr", "ifeq", "IF", "If"]);
            let expr = *rng.pick(&["1", "0", "1/0", "1%0", "2^99999", "99999999999999999999*99999999999999999999", "-", "(", ")", "((((((((((", "1+", "abs(-9223372036854775808)", "-9223372036854775808/-1", "-9223372036854775808%-1", "max()", "min(1,", "1e309", "0.1+0.2", "round(1.5)", "floor(-0.5)", "ceil(1e308*10)", "1 < 2", "a", "true", "", "|", "1 | 2 | 3 | 4", "\u{0}", "İ", "9223372036854775807+1", "-(-9223372036854775808)", "0^-1", "10^-400", "2^0.5", "(-8)^(1/3)", "1e-320", "1e400-1e400", "sqrt(-1)", "log(0)", "1 mod 0", "5 mod -0", "trunc(1e30)", "1 and", "not", "!", "0x", "1e", "1e+", "1.e5", ".5", "-.5e-"]);
            let nest = if rng.chance(1, 4) { depth.min(1100) } else { 1 };
            let open = format!("[[#{f} {expr} |").repeat(nest);
            let close = format!(" yes | no ]]").repeat(nest - rng.below(nest + 1).min(nest));
            format!("{open}{text}{close}")
        }
        16 => {
            let fmt = *rng.pick(&["%", "%%", "%Y", "%E", "%O", "%-", "%_", "%0", "%^", "%#", "%e%f%g%h%i%j%k%l%m%n%o%p%q%r%s%t%u%v%w%x%y%z", "%A%B%C%D%F%G%H%I%J%K%L%M%N%O%P%Q%R%S%T%U%V%W%X%Y%Z", "%-d", "%_d", "%0d", "%-", "%9999999999d", "%:z", "%::z", "%:::z", "%::::z", "%.3f", "%3f", "%.f", "İ%Y", "%\u{301}", "%🦀", "%\0", "%%%%%"]);
            let ts = *rng.pick(&["0", "-1", "1", "253402300799", "253402300800", "-62167219200", "-62167219201", "9223372036854775807", "-9223372036854775808", "99999999999999999999", "1e10", "x", "", "2026-11-05", "2026-13-45", "now", "today", "-", "+5", "0001-01-01", "9999-12-31T23:59:59Z", "10000-01-01", "-0001-01-01", "1970-01-01T00:00:00+99:99", "1700000000.5", "1700000000 1700000001"]);
            format!("[[date {ts} format=\"{fmt}\"{args}]]")
        }
        17 => {
            // Footnotes / bibliography cross nesting.
            let mut s = String::new();
            for _ in 0..depth.min(200) {
                s.push_str(rng.pick(&["[[footnote]]", "[[bibliography]]", ": x : y", "[[bibcite x]]", "[[footnoteblock]]", "[[/footnote]]", "[[/bibliography]]", "[[math x]]", "[[eref x]]", "[[toc]]", "+ h", "++++++ h", "+++++++ h", eol]));
                s.push_str(&text);
            }
            s
        }
        18 => {
            let w = *rng.pick(&["0", "-1", "1", "99999999999999999999", "100%", "1e9", "-0", "NaN", "", "a", "50%%", "-50%", "4294967296", "65536", "%"]);
            let kind = *rng.pick(&["image", "iframe https://x", "embedvideo", "gallery", "size", "audio x", "video x", "file x", "lines", "newlines", "collapsible", "div", "span", "table", "cell", "image x.png size=", "user", "radio", "checkbox", "button", "social", "toc", "f>image x"]);
            format!("[[{kind} width=\"{w}\" height=\"{w}\" size=\"{w}\" {w}]]{eol}{text}{eol}[[/{}]]", kind.split(' ').next().unwrap_or(""))
        }
        19 => {
            let mut s = String::new();
            for _ in 0..depth.min(2000) {
                s.push_str(rng.pick(&["[[ul]]", "[[ol]]", "[[li]]", "[[/li]]", "[[/ul]]", "[[/ol]]", "* ", "# ", " * ", "  # ", eol]));
            }
            s.push_str(&text);
            s
        }
        20 => {
            let mut s = String::new();
            for _ in 0..depth.min(2000) {
                s.push_str(rng.pick(&["[[ruby]]", "[[rt]]", "[[/rt]]", "[[/ruby]]", "[[rb]]", "[[/rb]]", "[[ruby2 x]]", "x", "|", eol]));
            }
            s
        }
        21 => {
            let mut s = String::new();
            for _ in 0..depth.min(2000) {
                s.push_str(rng.pick(&["[[iftags +a]]", "[[iftags -a]]", "[[/iftags]]", "[[ifcategory x]]", "[[/ifcategory]]", "[[hidden]]", "[[/hidden]]", "[[later]]", "[[/later]]", "[[raw]]", "[[/raw]]", "@@", "[!--", "--]", "[[html]]", "[[/html]]", "[[code]]", "[[/code]]", eol]));
            }
            s
        }
        22 => {
            // include syntax variants
            let page = *rng.pick(&["x", "", ":", "x:y", ":x:y", "İ", "\u{0}", "%", "a b", "x|", "x |a=b", "x | a = b | c = {$d}", "x\n|a=", "x|a=b|a=c", "x |", "|", "x | | |", "x |=", "x | =y"]);
            let pre = *rng.pick(&["", ">", "> ", ">>", " ", "\t>"]);
            format!("{pre}[[include {page}]]{eol}{pre}[[include{args} {page}{eol}{pre}]]{text}")
        }
        23 => {
            // CR/LF/NUL soup around a structure.
            let parts = ["\r", "\n", "\r\n", "\0", "\u{2028}", "\u{85}", "\u{FEFF}", " ", "\t", "\u{B}", "\u{C}"];
            let mut s = String::new();
            for _ in 0..(1 + rng.below(20)) {
                s.push_str(rng.pick(&parts));
                s.push_str(&rand_fragment(rng));
            }
            s
        }
        24 => {
            // Links of all forms with odd targets.
            let t = *rng.pick(&["", "*", "#", "/", "//", "http://", "https://x", "x:y", ":", "::", "İ", "\u{0}", "x#", "#x", "x|", "|", "]", "[", "javascript:x", "mailto:", "x y", " ", "%", "%%", "*https://x", "*#x"]);
            match rng.below(6) {
                0 => format!("[[[{t}]]]"),
                1 => format!("[[[{t}|{text}]]]"),
                2 => format!("[{t} {text}]"),
                3 => format!("[[a href=\"{t}\"]]{text}[[/a]]"),
                4 => format!("[[image {t} link=\"{t}\"]]"),
                _ => format!("{t}{text}"),
            }
        }
        _ => {
            let mut s = String::new();
            for _ in 0..(1 + rng.below(30)) {
                s.push_str(&rand_fragment(rng));
            }
            s
        }
    }
}
