/*
 * tree/variables.rs
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

use super::clone::string_map_to_owned;
use std::borrow::Cow;
use std::collections::BTreeMap;

pub type VariableMap<'t> = BTreeMap<Cow<'t, str>, Cow<'t, str>>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariableScopes {
    scopes: Vec<VariableMap<'static>>,
}

impl VariableScopes {
    #[inline]
    pub fn new() -> Self {
        VariableScopes::default()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }

        None
    }

    pub fn push_scope(&mut self, scope: &VariableMap) {
        self.scopes.push(string_map_to_owned(scope));
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop().expect("Scope stack was empty");
    }
}

#[test]
fn variables_serialize_in_canonical_order() {
    let mut variables = VariableMap::new();
    variables.insert(Cow::Borrowed("name"), Cow::Borrowed("peanut.png"));
    variables.insert(Cow::Borrowed("caption"), Cow::Borrowed("haha funi!"));
    variables.insert(Cow::Borrowed("width"), Cow::Borrowed("100"));

    let serialized = serde_json::to_string(&variables).expect("serialize variables");
    let caption = serialized.find("\"caption\"").unwrap();
    let name = serialized.find("\"name\"").unwrap();
    let width = serialized.find("\"width\"").unwrap();
    assert!(caption < name && name < width);
}
