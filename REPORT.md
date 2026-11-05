# Public API panic fuzzing report

Status: work in progress.

## Scope

Adversarial inputs are fed through the public entry points:

- `ftml::preprocess`, `ftml::preprocess_for_layout`
- `ftml::resolve_wikidot_parser_functions(_with_options)`
- `ftml::include` with `DebugIncluder` / `NullIncluder`
- `ftml::tokenize`, `ftml::parse` (every `WikitextMode`, both layouts)
- `HtmlRender`, `TextRender`, `DebugRender`, `NullRender`
- the delayed List pipeline (`DelayedInput::new`, `parse_delayed_list`,
  `bind`, `render_html`)
- `Utf16IndexMap`

## Findings

(in progress)
