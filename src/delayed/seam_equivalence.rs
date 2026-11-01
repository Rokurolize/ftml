/*
 * delayed/seam_equivalence.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2026 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

//! Exhaustive check that suppression seams are transparent to Wikidot typography.
//!
//! Property: take authored text segments separated by typography boundaries (seams).
//! Typographically substitute each segment alone, as the delayed pipeline does for
//! authored segments, then resolve the seams. The joined text of the result must equal
//! `substitute_wikidot` run on the plain concatenation of the raw segments, as if the
//! suppressed content had never been in the source. It must also not panic and must
//! leave no seam behind. Run through `verification/seam-typography/run.sh`; see the
//! `verify-seam-typography` skill. The test is `#[ignore]`.

use super::seam::resolve_static_suppressions;
use crate::delayed::DelayedElement;
use crate::preproc::typography::substitute_wikidot;
use crate::tree::Element;
use std::borrow::Cow;

/// Text tokens; `None` in a sequence is a seam. Dash runs are excluded: they become
/// separate dash elements, not text.
const TOKENS: &[&str] = &[
    "\"", "'", "a", " ", ".", "...", "<", ">", "<<", ">>", "\u{e9}",
];

fn typography(raw: &str) -> String {
    let mut text = raw.to_owned();
    substitute_wikidot(&mut text);
    text
}

/// Names a mismatch by the characters on which the two strings disagree (as multisets),
/// so one root cause is one class however many inputs reach it.
fn difference_class(got: &str, expected: &str) -> String {
    let mut counts: std::collections::BTreeMap<char, i64> =
        std::collections::BTreeMap::new();
    for c in got.chars() {
        *counts.entry(c).or_default() += 1;
    }
    for c in expected.chars() {
        *counts.entry(c).or_default() -= 1;
    }
    let mut class = String::from("text-differs:");
    for (c, n) in counts {
        if n != 0 {
            class.push_str(&format!(" {c}{}", if n > 0 { "+" } else { "-" }));
        }
    }
    class
}

/// `pick` indexes TOKENS, with `TOKENS.len()` meaning a seam.
fn check(pick: &[usize]) -> Option<(String, String)> {
    let seam = TOKENS.len();
    let mut segments: Vec<String> = vec![String::new()];
    for &p in pick {
        if p == seam {
            segments.push(String::new());
        } else {
            segments.last_mut().unwrap().push_str(TOKENS[p]);
        }
    }
    let describe = || {
        segments
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(" | ")
    };

    let mut elements: Vec<Element<'static>> = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            elements.push(Element::Delayed(DelayedElement::typography_boundary()));
        }
        if !segment.is_empty() {
            elements.push(Element::Text(Cow::Owned(typography(segment))));
        }
    }
    let had_seam = segments.len() > 1;

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        resolve_static_suppressions(&mut elements, true);
        elements
    }));
    let Ok(elements) = outcome else {
        return Some(("panic".into(), format!("segments {}: panicked", describe())));
    };
    if elements.iter().any(|e| matches!(e, Element::Delayed(_))) {
        return Some((
            "seam-survived".into(),
            format!("segments {}: a seam survived", describe()),
        ));
    }
    let mut got = String::new();
    for element in &elements {
        match element {
            Element::Text(text) => got.push_str(text),
            other => {
                return Some((
                    "unexpected-element".into(),
                    format!("segments {}: unexpected element {other:?}", describe()),
                ));
            }
        }
    }
    let expected = typography(&segments.concat());
    if got != expected {
        let _ = had_seam;
        return Some((
            difference_class(&got, &expected),
            format!(
                "segments {}: got {got:?} but typography of the joined text is {expected:?}",
                describe()
            ),
        ));
    }
    None
}

#[test]
#[ignore = "run through verification/seam-typography/run.sh"]
fn seams_are_transparent_to_wikidot_typography() {
    let max_tokens: usize = std::env::var("FTML_SEAM_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let choices = TOKENS.len() + 1;
    std::panic::set_hook(Box::new(|_| {}));
    let mut checked = 0u64;
    let mut shortest: std::collections::BTreeMap<String, (usize, u64, String)> =
        std::collections::BTreeMap::new();
    let mut failure_count = 0u64;
    for len in 0..=max_tokens {
        let mut pick = vec![0usize; len];
        loop {
            checked += 1;
            if let Some((class, message)) = check(&pick) {
                failure_count += 1;
                shortest.entry(class).or_insert((len, 0, message)).1 += 1;
            }
            let mut pos = 0;
            while pos < len {
                pick[pos] += 1;
                if pick[pos] < choices {
                    break;
                }
                pick[pos] = 0;
                pos += 1;
            }
            if pos == len {
                break;
            }
        }
    }
    let _ = std::panic::take_hook();
    for (class, (len, count, example)) in &shortest {
        println!("CLASS {class}|{len}|{count}|{example}");
    }
    println!(
        "SEAM-BOUND max_tokens={max_tokens} checked={checked} failures={failure_count}"
    );
    assert!(
        shortest.is_empty(),
        "{failure_count} inputs broke the property in {} classes (listed above)",
        shortest.len()
    );
}
