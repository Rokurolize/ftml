/*
 * test/conf_names.rs
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

//! Keeps the rule registries and `conf/*.toml` agreeing on which names each
//! block and module accepts.

use std::collections::{BTreeMap, BTreeSet};

fn normalize(name: &str) -> String {
    name.replace('-', "").to_lowercase()
}

/// Reads `conf/<file>` as a flat map of entry key to accepted names.
///
/// Only top-level `[entry]` tables and their single-line `aliases = [...]` and
/// `exclude-name = true` keys are read, which is all the conf files use for names.
fn conf_names(
    file: &str,
    always_include_key: bool,
) -> BTreeMap<String, BTreeSet<String>> {
    let path = format!("{}/conf/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).expect("conf file readable");

    let mut entries: BTreeMap<String, (BTreeSet<String>, bool)> = BTreeMap::new();
    let mut current = None;

    for line in text.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            current = (!header.contains('.')).then(|| {
                entries.entry(header.to_owned()).or_default();
                header.to_owned()
            });
        } else if let Some(entry) = current.as_ref().and_then(|key| entries.get_mut(key))
        {
            if let Some(list) = line.strip_prefix("aliases") {
                let list = list.split('#').next().unwrap_or("");
                entry
                    .0
                    .extend(list.split('"').skip(1).step_by(2).map(normalize));
            } else if line.starts_with("exclude-name") {
                entry.1 = true;
            }
        }
    }

    entries
        .into_iter()
        .map(|(key, (mut names, excluded))| {
            if always_include_key || !excluded {
                names.insert(normalize(&key));
            }
            (normalize(&key), names)
        })
        .collect()
}

/// Asserts that `rules` (rule name, accepted names) matches the conf entries,
/// except for `known_missing` rules that are deliberately absent from the conf.
pub(crate) fn assert_rules_match_conf(
    file: &str,
    name_prefix: &str,
    always_include_key: bool,
    known_missing: &[&str],
    rules: impl Iterator<Item = (&'static str, &'static [&'static str])>,
) {
    let conf = conf_names(file, always_include_key);
    let rules: BTreeMap<String, BTreeSet<String>> = rules
        .map(|(name, accepts)| {
            let key =
                normalize(name.strip_prefix(name_prefix).expect("rule name prefix"));
            (key, accepts.iter().map(|n| normalize(n)).collect())
        })
        .collect();

    for name in known_missing {
        assert!(
            !conf.contains_key(&normalize(name)),
            "{name} is now in conf/{file}; remove it from the known-missing list",
        );
    }

    let known: BTreeSet<String> = known_missing.iter().map(|n| normalize(n)).collect();
    let rule_keys: BTreeSet<_> = rules.keys().filter(|k| !known.contains(*k)).collect();
    let conf_keys: BTreeSet<_> = conf.keys().collect();
    assert_eq!(
        rule_keys, conf_keys,
        "rule registry and conf/{file} list different entries"
    );

    for key in conf_keys {
        assert_eq!(
            rules[key], conf[key],
            "accepted names differ for {key} in conf/{file}"
        );
    }
}
