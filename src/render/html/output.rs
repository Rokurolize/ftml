/*
 * render/html/output.rs
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

use super::meta::HtmlMeta;
use crate::data::Backlinks;

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WikidotTabViewRequirement {
    id: String,
}

impl WikidotTabViewRequirement {
    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "requirement", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum HtmlResourceRequirement {
    WikidotTabView(WikidotTabViewRequirement),
}

impl HtmlResourceRequirement {
    pub(crate) fn wikidot_tab_view(id: String) -> Self {
        assert!(
            valid_wikidot_tab_view_id(&id),
            "tabview requirement id must be renderer-generated",
        );
        Self::WikidotTabView(WikidotTabViewRequirement { id })
    }

    pub fn wikidot_tab_view_requirement(&self) -> Option<&WikidotTabViewRequirement> {
        match self {
            Self::WikidotTabView(requirement) => Some(requirement),
        }
    }
}

fn valid_wikidot_tab_view_id(id: &str) -> bool {
    let Some(suffix) = id.strip_prefix("wiki-tabview-") else {
        return false;
    };
    suffix.len() == 32 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Serialize, Debug, Clone)]
pub struct HtmlOutput {
    pub body: String,
    pub meta: Vec<HtmlMeta>,
    pub styles: Vec<String>,
    pub resource_requirements: Vec<HtmlResourceRequirement>,
    pub backlinks: Backlinks<'static>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabview_requirement_serializes_only_its_typed_generated_id() {
        let id = "wiki-tabview-0123456789abcdef0123456789abcdef";
        let requirement = HtmlResourceRequirement::wikidot_tab_view(id.to_owned());

        assert_eq!(
            serde_json::to_value(&requirement).unwrap(),
            serde_json::json!({
                "type": "wikidot-tab-view",
                "requirement": { "id": id },
            }),
        );
        assert_eq!(requirement.wikidot_tab_view_requirement().unwrap().id(), id,);
    }

    #[test]
    #[should_panic(expected = "tabview requirement id must be renderer-generated")]
    fn tabview_requirement_rejects_authored_script_data() {
        HtmlResourceRequirement::wikidot_tab_view(
            "wiki-tabview-x');alert(1)//".to_owned(),
        );
    }
}
