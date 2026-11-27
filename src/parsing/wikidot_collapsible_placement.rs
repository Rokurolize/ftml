use crate::preproc::quote_depth_and_body;
use crate::tree::{
    AttributeMap, Container, ContainerType, Element, ListItem, ListType,
    WIKIDOT_HEADING_COLLAPSIBLE_BODY_MARKER, WIKIDOT_HEADING_COLLAPSIBLE_LINE_END_MARKER,
    WIKIDOT_HEADING_COLLAPSIBLE_TAIL_MARKER,
};

pub(crate) fn repair_wikidot_collapsible_placement<'t>(
    elements: &mut Vec<Element<'t>>,
    footnotes: &mut Vec<Vec<Element<'t>>>,
    source: &str,
    has_footnote_block: &mut bool,
) {
    retain_cross_line_empty_div_list(elements, source);
    let lower_source = source.to_ascii_lowercase();
    let has_collapsible = lower_source.contains("[[collapsible");
    let has_toc = lower_source.contains("[[toc");
    if has_toc && source_has_swallowed_toc_break(source) {
        insert_break_after_root_toc(elements);
    }
    if !has_collapsible && !has_toc {
        return;
    }

    if has_collapsible {
        visit_elements_mut(elements, unwrap_literal_div_tail_after_collapsible);
        visit_elements_mut(elements, drop_nested_collapsible_boundary_break);
    }
    if source.contains('>') {
        repair_quote_elements(elements, source, false);
    }
    if has_collapsible && source_has_nonempty_heading_collapsible(source) {
        visit_elements_mut(elements, flatten_nested_heading_label);
    }
    if has_collapsible {
        repair_footnote_shapes(elements, footnotes, has_footnote_block);
    }
}

fn visit_elements_mut<'t>(
    elements: &mut Vec<Element<'t>>,
    transform: fn(&mut Vec<Element<'t>>),
) {
    transform(elements);
    for element in elements {
        match element {
            Element::Container(container) => {
                visit_elements_mut(container.elements_mut(), transform)
            }
            Element::Anchor { elements, .. }
            | Element::Collapsible { elements, .. }
            | Element::Color { elements, .. }
            | Element::Include { elements, .. } => {
                visit_elements_mut(elements, transform)
            }
            Element::WikidotBlockFrame { children, .. } => {
                visit_elements_mut(children, transform)
            }
            Element::List { items, .. } => {
                for item in items {
                    match item {
                        ListItem::Elements { elements, .. } => {
                            visit_elements_mut(elements, transform)
                        }
                        ListItem::SubList { .. } => {}
                    }
                }
            }
            Element::DefinitionList(items) => {
                for item in items {
                    visit_elements_mut(&mut item.key_elements, transform);
                    visit_elements_mut(&mut item.value_elements, transform);
                }
            }
            Element::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        visit_elements_mut(&mut cell.elements, transform);
                    }
                }
            }
            Element::TabView(items) => {
                for item in items {
                    visit_elements_mut(item.elements_mut(), transform);
                }
            }
            _ => {}
        }
    }
}

fn unwrap_literal_div_tail_after_collapsible<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index + 1 < elements.len() {
        let unwrap = matches!(elements[index], Element::Collapsible { .. })
            && matches!(
                &elements[index + 1],
                Element::Container(container)
                    if container.ctype() == ContainerType::Paragraph
                        && paragraph_is_literal_div_pair(container.elements())
            );
        if unwrap {
            let Element::Container(container) = elements.remove(index + 1) else {
                unreachable!();
            };
            let children: Vec<Element<'t>> = container.into();
            let count = children.len();
            elements.splice(index + 1..index + 1, children);
            index += count + 1;
        } else {
            index += 1;
        }
    }
}

fn paragraph_is_literal_div_pair(elements: &[Element<'_>]) -> bool {
    elements.len() >= 2
        && matches!(
            elements.first(),
            Some(Element::Text(text)) if text.eq_ignore_ascii_case("[[div]]")
        )
        && elements[1..]
            .iter()
            .any(|element| matches!(element, Element::Text(text) if text.eq_ignore_ascii_case("[[/div]]")))
}

fn drop_nested_collapsible_boundary_break<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index + 1 < elements.len() {
        let drop = match (&elements[index], &elements[index + 1]) {
            (Element::Collapsible { elements: body, .. }, Element::LineBreak) => {
                let mut text = String::new();
                for element in body {
                    collect_element_text(element, &mut text);
                }
                text.to_ascii_lowercase().contains("[[collapsible]]")
            }
            _ => false,
        };
        if drop {
            elements.remove(index + 1);
        }
        index += 1;
    }
}

fn collect_element_text(element: &Element<'_>, output: &mut String) {
    match element {
        Element::Text(text) | Element::Raw(text) => output.push_str(text),
        Element::Container(container) => {
            for child in container.elements() {
                collect_element_text(child, output);
            }
        }
        Element::Anchor { elements, .. }
        | Element::Collapsible { elements, .. }
        | Element::Color { elements, .. }
        | Element::Include { elements, .. } => {
            for child in elements {
                collect_element_text(child, output);
            }
        }
        Element::WikidotBlockFrame { children, .. } => {
            for child in children {
                collect_element_text(child, output);
            }
        }
        _ => {}
    }
}

fn flatten_nested_heading_label<'t>(elements: &mut Vec<Element<'t>>) {
    for element in elements {
        let Element::Container(header) = element else {
            continue;
        };
        if !matches!(header.ctype(), ContainerType::Header(_)) {
            continue;
        }
        let replacement = if header.elements().len() == 1 {
            match &header.elements()[0] {
                Element::Container(span)
                    if span.ctype() == ContainerType::Span
                        && span.elements().len() == 1 =>
                {
                    match &span.elements()[0] {
                        Element::Container(label)
                            if label.ctype() == ContainerType::HeadingLabel =>
                        {
                            Some(Element::Container(label.clone()))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            None
        };
        if let Some(replacement) = replacement {
            header.elements_mut()[0] = replacement;
        }
    }
}

fn source_has_nonempty_heading_collapsible(source: &str) -> bool {
    for line in normalized_lines(source) {
        let line = line.trim_start();
        let plus_count = line.bytes().take_while(|byte| *byte == b'+').count();
        if plus_count == 0 {
            continue;
        }
        let rest = line[plus_count..].trim_start().to_ascii_lowercase();
        if !rest.starts_with("[[collapsible") {
            continue;
        }
        let Some(open_close) = rest.find("]]") else {
            continue;
        };
        let head = &rest[..open_close + 2];
        if head != "[[collapsible]]" && !head.starts_with("[[collapsible ") {
            continue;
        }
        let after_open = &rest[open_close + 2..];
        let Some(close_start) = after_open.find("[[/collapsible]]") else {
            continue;
        };
        if !after_open[..close_start].trim().is_empty() {
            return true;
        }
    }
    false
}

fn retain_cross_line_empty_div_list<'t>(elements: &mut Vec<Element<'t>>, source: &str) {
    let Some(ltype) = cross_line_empty_div_list_type(source) else {
        return;
    };
    if matches!(elements.first(), Some(Element::List { .. })) {
        return;
    }
    elements.insert(
        0,
        Element::List {
            ltype,
            items: vec![ListItem::Elements {
                elements: Vec::new(),
                attributes: AttributeMap::new(),
            }],
            attributes: AttributeMap::new(),
            wikidot_explicit_block: false,
            wikidot_marker_head_spawn: false,
            wikidot_unclosed: false,
        },
    );
}

fn cross_line_empty_div_list_type(source: &str) -> Option<ListType> {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.lines();
    let first = lines.next()?.trim_end_matches([' ', '\t']);
    let ltype = match first {
        "# [[div]]" => ListType::Numbered,
        "* [[div]]" => ListType::Bullet,
        _ => return None,
    };
    let second = lines.next().unwrap_or("");
    second
        .to_ascii_lowercase()
        .starts_with("[[/div]]")
        .then_some(ltype)
}

fn source_has_swallowed_toc_break(source: &str) -> bool {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = normalized.lines().collect();
    for index in 0..lines.len().saturating_sub(1) {
        let line = lines[index];
        let lower = line.to_ascii_lowercase();
        if !lower.starts_with("[[toc")
            || !lower[5..]
                .chars()
                .next()
                .is_some_and(|ch| ch == ']' || ch.is_whitespace())
        {
            continue;
        }
        let Some(close) = lower.find("]]") else {
            continue;
        };
        let after = &line[close + 2..];
        if after.is_empty() || !after.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
            continue;
        }
        let next = lines[index + 1];
        let definition = next.strip_prefix(": ").or_else(|| next.strip_prefix(":\t"));
        let Some(term) = definition else {
            continue;
        };
        if term.trim().is_empty() {
            continue;
        }
        if lines[index + 2..].iter().all(|line| line.trim().is_empty()) {
            return true;
        }
    }
    false
}

fn insert_break_after_root_toc<'t>(elements: &mut Vec<Element<'t>>) {
    let Some(index) = elements
        .iter()
        .position(|element| matches!(element, Element::TableOfContents { .. }))
    else {
        return;
    };
    if matches!(elements.get(index + 1), Some(Element::LineBreak)) {
        return;
    }
    elements.insert(index + 1, Element::LineBreak);
}

fn normalized_lines(source: &str) -> Vec<String> {
    source
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(str::to_owned)
        .collect()
}

fn source_has_quote_text_then_collapsible(source: &str) -> bool {
    let lines = normalized_lines(source);
    for pair in lines.windows(2) {
        let (first_depth, first) = quote_depth_and_body(&pair[0]);
        let (second_depth, second) = quote_depth_and_body(&pair[1]);
        if first_depth == 0 || second_depth != first_depth {
            continue;
        }
        let first_trimmed = first.trim_start();
        let second = second.trim_start().to_ascii_lowercase();
        if first.trim().is_empty()
            || first_trimmed.starts_with(['#', '*', '+', '=', ':', ';'])
            || first_trimmed.starts_with("[[")
            || empty_collapsible_tail(&second).is_none()
        {
            continue;
        }
        return true;
    }
    false
}

fn source_has_literal_quote_collapsible_continuation(source: &str) -> bool {
    let lines = normalized_lines(source);
    for pair in lines.windows(2) {
        let first = pair[0].trim_start();
        let second = pair[1].trim_start().to_ascii_lowercase();
        if first.starts_with("> ")
            && !first.starts_with(">>")
            && first.len() > 2
            && second
                .strip_prefix("> > ")
                .is_some_and(|rest| empty_collapsible_tail(rest).is_some())
        {
            return true;
        }
    }
    false
}

fn empty_collapsible_tail(rest: &str) -> Option<&str> {
    if !rest.starts_with("[[collapsible") {
        return None;
    }
    let open_close = rest.find("]]")?;
    let head = &rest[..open_close + 2];
    let valid_head = head == "[[collapsible]]"
        || (head.starts_with("[[collapsible ")
            && head.ends_with("]]")
            && head.len() > "[[collapsible ]]".len());
    if !valid_head {
        return None;
    }
    rest[open_close + 2..].strip_prefix("[[/collapsible]]")
}

fn source_has_quote_list_empty_collapsible_tail(source: &str) -> bool {
    for line in normalized_lines(source) {
        let (depth, body) = quote_depth_and_body(&line);
        if depth == 0 {
            continue;
        }
        let body = body.trim_start();
        let Some(rest) = body
            .strip_prefix('#')
            .or_else(|| body.strip_prefix('*'))
            .map(str::trim_start)
        else {
            continue;
        };
        let lower = rest.to_ascii_lowercase();
        if empty_collapsible_tail(&lower).is_some_and(|tail| !tail.trim().is_empty()) {
            return true;
        }
    }
    false
}

fn source_has_quoted_nonempty_collapsible_tail(source: &str) -> bool {
    for line in normalized_lines(source) {
        let (depth, body) = quote_depth_and_body(&line);
        if depth == 0 {
            continue;
        }
        let mut body = body.trim_start();
        if let Some(rest) = body
            .strip_prefix('#')
            .or_else(|| body.strip_prefix('*'))
            .map(str::trim_start)
        {
            body = rest;
        }
        let lower = body.to_ascii_lowercase();
        if !lower.starts_with("[[collapsible") {
            continue;
        }
        let Some(open_close) = lower.find("]]") else {
            continue;
        };
        let head = &lower[..open_close + 2];
        if head != "[[collapsible]]" && !head.starts_with("[[collapsible ") {
            continue;
        }
        let after_open = &lower[open_close + 2..];
        let Some(close_start) = after_open.find("[[/collapsible]]") else {
            continue;
        };
        let inline_body = &after_open[..close_start];
        let tail = &after_open[close_start + "[[/collapsible]]".len()..];
        if !inline_body.trim().is_empty() && !tail.trim().is_empty() {
            return true;
        }
    }
    false
}

#[derive(Copy, Clone)]
struct QuoteRepairFlags {
    empty_list_tail: bool,
    nonempty_collapsible_tail: bool,
    text_then_collapsible: bool,
    literal_continuation: bool,
}

fn repair_quote_elements<'t>(
    elements: &mut Vec<Element<'t>>,
    source: &str,
    in_quote: bool,
) {
    let flags = QuoteRepairFlags {
        empty_list_tail: source_has_quote_list_empty_collapsible_tail(source),
        nonempty_collapsible_tail: source_has_quoted_nonempty_collapsible_tail(source),
        text_then_collapsible: source_has_quote_text_then_collapsible(source),
        literal_continuation: source_has_literal_quote_collapsible_continuation(source),
    };
    repair_quote_elements_inner(elements, in_quote, flags);
}

fn repair_quote_elements_inner<'t>(
    elements: &mut Vec<Element<'t>>,
    in_quote: bool,
    flags: QuoteRepairFlags,
) {
    if in_quote {
        repair_quote_vector(elements, flags);
    }
    for element in &mut *elements {
        let enters_quote = matches!(
            element,
            Element::Container(container) if container.ctype() == ContainerType::Blockquote
        );
        let child_in_quote = in_quote || enters_quote;
        match element {
            Element::Container(container) => repair_quote_elements_inner(
                container.elements_mut(),
                child_in_quote,
                flags,
            ),
            Element::Anchor { elements, .. }
            | Element::Collapsible { elements, .. }
            | Element::Color { elements, .. }
            | Element::Include { elements, .. } => {
                repair_quote_elements_inner(elements, child_in_quote, flags)
            }
            Element::WikidotBlockFrame { children, .. } => {
                repair_quote_elements_inner(children, child_in_quote, flags)
            }
            Element::List { items, .. } => {
                for item in items {
                    match item {
                        ListItem::Elements { elements, .. } => {
                            repair_quote_elements_inner(elements, child_in_quote, flags)
                        }
                        ListItem::SubList { .. } => {}
                    }
                }
            }
            Element::DefinitionList(items) => {
                for item in items {
                    repair_quote_elements_inner(
                        &mut item.key_elements,
                        child_in_quote,
                        flags,
                    );
                    repair_quote_elements_inner(
                        &mut item.value_elements,
                        child_in_quote,
                        flags,
                    );
                }
            }
            Element::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        repair_quote_elements_inner(
                            &mut cell.elements,
                            child_in_quote,
                            flags,
                        );
                    }
                }
            }
            Element::TabView(items) => {
                for item in items {
                    repair_quote_elements_inner(
                        item.elements_mut(),
                        child_in_quote,
                        flags,
                    );
                }
            }
            _ => {}
        }
    }
}

fn repair_quote_vector<'t>(elements: &mut Vec<Element<'t>>, flags: QuoteRepairFlags) {
    if flags.empty_list_tail {
        relocate_empty_collapsible_list_tail(elements);
    }
    if flags.nonempty_collapsible_tail {
        restore_quoted_collapsible_bodies(elements);
    }
    unwrap_heading_tail_paragraphs(elements);
    paragraph_heading_body_continuation(elements);
    if flags.text_then_collapsible {
        wrap_text_before_collapsible(elements);
    }
    if flags.literal_continuation {
        unwrap_literal_continuation_paragraph(elements);
    }
    wrap_toc_tails(elements);
}

fn inline_tail_element(element: &Element<'_>) -> bool {
    element.paragraph_safe()
        && !matches!(element, Element::LineBreak | Element::LineBreaks(_))
}

fn relocate_empty_collapsible_list_tail<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index < elements.len() {
        let mut tail = None;
        if let Element::List { items, .. } = &mut elements[index]
            && items.len() == 1
            && let ListItem::Elements {
                elements: item_elements,
                ..
            } = &mut items[0]
            && item_elements.len() == 1
            && let Some(Element::Collapsible {
                elements: collapsible_elements,
                ..
            }) = item_elements.first_mut()
            && collapsible_elements.len() == 1
            && matches!(
                collapsible_elements.first(),
                Some(Element::Container(container))
                    if container.ctype() == ContainerType::Paragraph
            )
        {
            let Element::Container(container) = collapsible_elements.remove(0) else {
                unreachable!();
            };
            tail = Some(container.elements().to_vec());
        }
        if let Some(tail) = tail {
            elements.splice(index + 1..index + 1, tail);
            index += 1;
        }
        index += 1;
    }
}

fn restore_quoted_collapsible_bodies<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index < elements.len() {
        let empty_collapsible = matches!(
            &elements[index],
            Element::Collapsible { elements: body, .. } if body.is_empty()
        );
        if !empty_collapsible {
            index += 1;
            continue;
        }
        let newline = matches!(
            elements.get(index + 1),
            Some(Element::Text(text)) if text.as_ref() == "\n"
        );
        let body_index = index + 1 + usize::from(newline);
        let inline_count = elements[body_index..]
            .iter()
            .take_while(|element| inline_tail_element(element))
            .count();
        if inline_count >= 2 {
            let body = elements.remove(body_index);
            if newline {
                elements.remove(index + 1);
            }
            if let Element::Collapsible {
                elements: collapsible_elements,
                ..
            } = &mut elements[index]
            {
                *collapsible_elements = vec![Element::Container(Container::new(
                    ContainerType::Paragraph,
                    vec![body],
                    AttributeMap::new(),
                ))];
            }
        }
        index += 1;
    }
}

fn unwrap_heading_tail_paragraphs<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index < elements.len() {
        let unwrap = matches!(
            &elements[index],
            Element::Container(container)
                if container.ctype() == ContainerType::Paragraph
                    && container
                        .attributes()
                        .get()
                        .contains_key(WIKIDOT_HEADING_COLLAPSIBLE_TAIL_MARKER)
        );
        if unwrap {
            let Element::Container(container) = elements.remove(index) else {
                unreachable!();
            };
            let children: Vec<Element<'t>> = container.into();
            let count = children.len();
            elements.splice(index..index, children);
            index += count;
        } else {
            index += 1;
        }
    }
}

fn paragraph_heading_body_continuation<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 1;
    while index < elements.len() {
        let convert = matches!(
            &elements[index - 1],
            Element::Container(container)
                if container.ctype() == ContainerType::Paragraph
                    && container
                        .attributes()
                        .get()
                        .contains_key(WIKIDOT_HEADING_COLLAPSIBLE_BODY_MARKER)
                    && container
                        .attributes()
                        .get()
                        .contains_key(WIKIDOT_HEADING_COLLAPSIBLE_LINE_END_MARKER)
        ) && matches!(
            &elements[index],
            Element::Container(container)
                if container.ctype() == ContainerType::Span
                    && matches!(container.elements().first(), Some(Element::LineBreak))
        );
        if convert {
            let Element::Container(container) = elements.remove(index) else {
                unreachable!();
            };
            let mut children: Vec<Element<'t>> = container.into();
            children.remove(0);
            if matches!(children.first(), Some(Element::Text(text)) if text.starts_with('\n'))
            {
                let Some(Element::Text(text)) = children.first_mut() else {
                    unreachable!();
                };
                text.to_mut().remove(0);
                if text.is_empty() {
                    children.remove(0);
                }
            }
            elements.insert(
                index,
                Element::Container(Container::new(
                    ContainerType::Paragraph,
                    children,
                    AttributeMap::new(),
                )),
            );
        }
        index += 1;
    }
}

fn unwrap_literal_continuation_paragraph<'t>(elements: &mut Vec<Element<'t>>) {
    if elements.len() < 2 {
        return;
    }
    let unwrap = matches!(
        (&elements[0], &elements[1]),
        (
            Element::Container(container),
            Element::Text(marker),
        ) if container.ctype() == ContainerType::Paragraph && marker.as_ref() == ">"
    );
    if !unwrap {
        return;
    }
    let Element::Container(container) = elements.remove(0) else {
        unreachable!();
    };
    let children: Vec<Element<'t>> = container.into();
    let count = children.len();
    elements.splice(0..0, children);
    elements.insert(count, Element::LineBreak);
}

fn wrap_text_before_collapsible<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 1;
    while index < elements.len() {
        if matches!(elements[index], Element::Collapsible { .. })
            && elements.get(index - 1).is_some_and(inline_tail_element)
        {
            let mut start = index - 1;
            while start > 0 && inline_tail_element(&elements[start - 1]) {
                start -= 1;
            }
            let tail: Vec<Element<'t>> = elements.splice(start..index, []).collect();
            elements.insert(
                start,
                Element::Container(Container::new(
                    ContainerType::Paragraph,
                    tail,
                    AttributeMap::new(),
                )),
            );
            index = start + 1;
        }
        index += 1;
    }
}

fn wrap_toc_tails<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index + 1 < elements.len() {
        if matches!(elements[index], Element::TableOfContents { .. })
            && inline_tail_element(&elements[index + 1])
        {
            let mut end = index + 1;
            while end < elements.len() && inline_tail_element(&elements[end]) {
                end += 1;
            }
            let tail: Vec<Element<'t>> = elements.splice(index + 1..end, []).collect();
            elements.insert(
                index + 1,
                Element::Container(Container::new(
                    ContainerType::Paragraph,
                    tail,
                    AttributeMap::new(),
                )),
            );
        }
        index += 1;
    }
}

fn repair_footnote_shapes<'t>(
    root: &mut Vec<Element<'t>>,
    footnotes: &mut Vec<Vec<Element<'t>>>,
    has_footnote_block: &mut bool,
) {
    let mut external = Vec::new();
    for footnote in footnotes {
        nest_list_collapsible_body(footnote);
        external.extend(eject_heading_collapsible_tails(footnote));
        external.extend(eject_empty_collapsible_host_tail(footnote));
    }
    if external.is_empty() {
        return;
    }
    if let Some(index) = root
        .iter()
        .position(|element| matches!(element, Element::FootnoteBlock { .. }))
    {
        external.reverse();
        for element in external {
            root.insert(index + 1, element);
        }
    } else {
        root.push(Element::FootnoteBlock {
            title: None,
            hide: false,
            wikidot_wrappers: Vec::new(),
            wikidot_inline_suffix: false,
            wikidot_inline_prefix: false,
            wikidot_tail_attached: false,
        });
        root.extend(external);
        *has_footnote_block = true;
    }
}

fn nest_list_collapsible_body<'t>(elements: &mut Vec<Element<'t>>) {
    let mut index = 0;
    while index + 1 < elements.len() {
        let move_body = matches!(
            &elements[index],
            Element::List { .. } if element_has_empty_collapsible_content(&elements[index])
        ) && elements[..index].iter().any(element_has_visible_text)
            && matches!(
                &elements[index + 1],
                Element::Container(container)
                    if container.ctype() == ContainerType::Paragraph
            );
        if move_body {
            let paragraph = elements.remove(index + 1);
            if !push_into_empty_collapsible_content(
                &mut elements[index],
                paragraph.clone(),
            ) {
                elements.insert(index + 1, paragraph);
            }
        }
        index += 1;
    }
}

fn eject_heading_collapsible_tails<'t>(
    elements: &mut Vec<Element<'t>>,
) -> Vec<Element<'t>> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < elements.len() {
        let eject = matches!(
            &elements[index],
            Element::Container(container)
                if container.ctype() == ContainerType::Paragraph
                    && container
                        .attributes()
                        .get()
                        .contains_key(WIKIDOT_HEADING_COLLAPSIBLE_TAIL_MARKER)
        ) && elements[..index].iter().any(element_has_visible_text);
        if eject {
            let Element::Container(container) = elements.remove(index) else {
                unreachable!();
            };
            let children: Vec<Element<'t>> = container.into();
            output.extend(children);
        } else {
            index += 1;
        }
    }
    output
}

fn eject_empty_collapsible_host_tail<'t>(
    elements: &mut Vec<Element<'t>>,
) -> Vec<Element<'t>> {
    let mut output = Vec::new();
    let mut index = 0;
    while index + 1 < elements.len() {
        let has_host = matches!(
            &elements[index],
            Element::List { .. } if element_has_empty_collapsible_content(&elements[index])
        ) && elements[..index].iter().any(element_has_visible_text);
        if has_host {
            let mut tail = Vec::new();
            while index + 1 < elements.len() && inline_tail_element(&elements[index + 1])
            {
                tail.push(elements.remove(index + 1));
            }
            output.extend(tail);
        }
        index += 1;
    }
    output
}

fn element_has_visible_text(element: &Element<'_>) -> bool {
    match element {
        Element::Text(text) => !text.trim().is_empty(),
        Element::Container(container) => {
            container.elements().iter().any(element_has_visible_text)
        }
        Element::Anchor { elements, .. }
        | Element::Collapsible { elements, .. }
        | Element::Color { elements, .. }
        | Element::Include { elements, .. } => {
            elements.iter().any(element_has_visible_text)
        }
        Element::WikidotBlockFrame { children, .. } => {
            children.iter().any(element_has_visible_text)
        }
        _ => false,
    }
}

fn element_has_empty_collapsible_content(element: &Element<'_>) -> bool {
    match element {
        Element::Collapsible { elements, .. } => elements.is_empty(),
        Element::WikidotBlockFrame {
            attributes,
            children,
            ..
        } => {
            let empty_content = attributes
                .get()
                .get("class")
                .is_some_and(|class| class.as_ref() == "collapsible-block-content")
                && children.is_empty();
            empty_content || children.iter().any(element_has_empty_collapsible_content)
        }
        Element::Container(container) => container
            .elements()
            .iter()
            .any(element_has_empty_collapsible_content),
        Element::List { items, .. } => items.iter().any(|item| match item {
            ListItem::Elements { elements, .. } => {
                elements.iter().any(element_has_empty_collapsible_content)
            }
            ListItem::SubList { element } => {
                element_has_empty_collapsible_content(element)
            }
        }),
        _ => false,
    }
}

fn push_into_empty_collapsible_content<'t>(
    element: &mut Element<'t>,
    paragraph: Element<'t>,
) -> bool {
    match element {
        Element::WikidotBlockFrame {
            attributes,
            children,
            ..
        } => {
            let empty_content = attributes
                .get()
                .get("class")
                .is_some_and(|class| class.as_ref() == "collapsible-block-content")
                && children.is_empty();
            if empty_content {
                children.push(paragraph);
                return true;
            }
            for child in children {
                if push_into_empty_collapsible_content(child, paragraph.clone()) {
                    return true;
                }
            }
            false
        }
        Element::Container(container) => {
            for child in container.elements_mut() {
                if push_into_empty_collapsible_content(child, paragraph.clone()) {
                    return true;
                }
            }
            false
        }
        Element::List { items, .. } => {
            for item in items {
                let pushed = match item {
                    ListItem::Elements { elements, .. } => {
                        elements.iter_mut().any(|element| {
                            push_into_empty_collapsible_content(
                                element,
                                paragraph.clone(),
                            )
                        })
                    }
                    ListItem::SubList { element } => {
                        push_into_empty_collapsible_content(element, paragraph.clone())
                    }
                };
                if pushed {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}
