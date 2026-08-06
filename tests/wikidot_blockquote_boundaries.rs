use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("blockquote-boundaries"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Blockquote boundaries"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn wikidot_blockquote_boundary_matrix_matches_live_dom() {
    for (case_id, source, expected) in [
        (
            r#"scout-line-owner-block-quote-code-opener-only"#,
            r#"> [[code]]
A
[[/code]]"#,
            r#"<blockquote><p>[[code]]</p></blockquote><p>A<br>
[[/code]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-code-all-marked"#,
            r#"> [[code]]
> A
> [[/code]]"#,
            r#"<blockquote><p>[[code]]<br>
A<br>
[[/code]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-code-opener-close-marked"#,
            r#"> [[code]]
A
> [[/code]]"#,
            r#"<blockquote><p>[[code]]</p></blockquote><p>A</p><blockquote><p>[[/code]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-div-opener-only"#,
            r#"> [[div]]
A
[[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-div-all-marked"#,
            r#"> [[div]]
> A
> [[/div]]"#,
            r#"<blockquote><div><p>A</p></div></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-div-opener-close-marked"#,
            r#"> [[div]]
A
> [[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-collapsible-opener-only"#,
            r#"> [[collapsible]]
A
[[/collapsible]]"#,
            r#"<blockquote><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-collapsible-all-marked"#,
            r#"> [[collapsible]]
> A
> [[/collapsible]]"#,
            r#"<blockquote><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"><p>A</p></div></div></div></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-collapsible-opener-close-marked"#,
            r#"> [[collapsible]]
A
> [[/collapsible]]"#,
            r#"<blockquote><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-css-opener-only"#,
            r#"> [[module CSS]]
x{}
[[/module]]"#,
            r#"<blockquote><p>[[module CSS]]</p></blockquote><p>x{}<br>
[[/module]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-css-all-marked"#,
            r#"> [[module CSS]]
> x{}
> [[/module]]"#,
            r#"<blockquote><p>[[module CSS]]<br>
x{}<br>
[[/module]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-css-opener-close-marked"#,
            r#"> [[module CSS]]
x{}
> [[/module]]"#,
            r#"<blockquote><p>[[module CSS]]</p></blockquote><p>x{}</p><blockquote><p>[[/module]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-unknown-opener-only"#,
            r#"> [[module FooBar]]"#,
            r#"<blockquote><p>[[module FooBar]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-unknown-all-marked"#,
            r#"> [[module FooBar]]"#,
            r#"<blockquote><p>[[module FooBar]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-unknown-opener-close-marked"#,
            r#"> [[module FooBar]]"#,
            r#"<blockquote><p>[[module FooBar]]</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-align-opener-only"#,
            r#"> [[=]]
A
[[/=]]"#,
            r#"<blockquote><div style="text-align: center;"></div></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-align-all-marked"#,
            r#"> [[=]]
> A
> [[/=]]"#,
            r#"<blockquote><div style="text-align: center;"><p>A</p></div></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-align-opener-close-marked"#,
            r#"> [[=]]
A
> [[/=]]"#,
            r#"<blockquote><div style="text-align: center;"></div></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote-table-opener-only"#,
            r#"> || A ||"#,
            r#"<blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-table-all-marked"#,
            r#"> || A ||"#,
            r#"<blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-table-opener-close-marked"#,
            r#"> || A ||"#,
            r#"<blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-heading-opener-only"#,
            r#"> + H"#,
            r#"<blockquote><h1 id="toc0"><span>H</span></h1></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-heading-all-marked"#,
            r#"> + H"#,
            r#"<blockquote><h1 id="toc0"><span>H</span></h1></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote-heading-opener-close-marked"#,
            r#"> + H"#,
            r#"<blockquote><h1 id="toc0"><span>H</span></h1></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-code-opener-only"#,
            r#">> [[code]]
A
[[/code]]"#,
            r#"<blockquote><blockquote><p>[[code]]</p></blockquote></blockquote><p>A<br>
[[/code]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-code-all-marked"#,
            r#">> [[code]]
>> A
>> [[/code]]"#,
            r#"<blockquote><blockquote><p>[[code]]<br>
A<br>
[[/code]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-code-opener-close-marked"#,
            r#">> [[code]]
A
>> [[/code]]"#,
            r#"<blockquote><blockquote><p>[[code]]</p></blockquote></blockquote><p>A</p><blockquote><blockquote><p>[[/code]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-div-opener-only"#,
            r#">> [[div]]
A
[[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-div-all-marked"#,
            r#">> [[div]]
>> A
>> [[/div]]"#,
            r#"<blockquote><blockquote><div><p>A</p></div></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-div-opener-close-marked"#,
            r#">> [[div]]
A
>> [[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-collapsible-opener-only"#,
            r#">> [[collapsible]]
A
[[/collapsible]]"#,
            r#"<blockquote><blockquote><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></blockquote></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-collapsible-all-marked"#,
            r#">> [[collapsible]]
>> A
>> [[/collapsible]]"#,
            r#"<blockquote><blockquote><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"><p>A</p></div></div></div></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-collapsible-opener-close-marked"#,
            r#">> [[collapsible]]
A
>> [[/collapsible]]"#,
            r#"<blockquote><blockquote><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></blockquote></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-css-opener-only"#,
            r#">> [[module CSS]]
x{}
[[/module]]"#,
            r#"<blockquote><blockquote><p>[[module CSS]]</p></blockquote></blockquote><p>x{}<br>
[[/module]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-css-all-marked"#,
            r#">> [[module CSS]]
>> x{}
>> [[/module]]"#,
            r#"<blockquote><blockquote><p>[[module CSS]]<br>
x{}<br>
[[/module]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-css-opener-close-marked"#,
            r#">> [[module CSS]]
x{}
>> [[/module]]"#,
            r#"<blockquote><blockquote><p>[[module CSS]]</p></blockquote></blockquote><p>x{}</p><blockquote><blockquote><p>[[/module]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-unknown-opener-only"#,
            r#">> [[module FooBar]]"#,
            r#"<blockquote><blockquote><p>[[module FooBar]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-unknown-all-marked"#,
            r#">> [[module FooBar]]"#,
            r#"<blockquote><blockquote><p>[[module FooBar]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-unknown-opener-close-marked"#,
            r#">> [[module FooBar]]"#,
            r#"<blockquote><blockquote><p>[[module FooBar]]</p></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-align-opener-only"#,
            r#">> [[=]]
A
[[/=]]"#,
            r#"<blockquote><blockquote><div style="text-align: center;"></div></blockquote></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-align-all-marked"#,
            r#">> [[=]]
>> A
>> [[/=]]"#,
            r#"<blockquote><blockquote><div style="text-align: center;"><p>A</p></div></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-align-opener-close-marked"#,
            r#">> [[=]]
A
>> [[/=]]"#,
            r#"<blockquote><blockquote><div style="text-align: center;"></div></blockquote></blockquote><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-table-opener-only"#,
            r#">> || A ||"#,
            r#"<blockquote><blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-table-all-marked"#,
            r#">> || A ||"#,
            r#"<blockquote><blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-table-opener-close-marked"#,
            r#">> || A ||"#,
            r#"<blockquote><blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-heading-opener-only"#,
            r#">> + H"#,
            r#"<blockquote><blockquote><h1 id="toc0"><span>H</span></h1></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-heading-all-marked"#,
            r#">> + H"#,
            r#"<blockquote><blockquote><h1 id="toc0"><span>H</span></h1></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-quote2-heading-opener-close-marked"#,
            r#">> + H"#,
            r#"<blockquote><blockquote><h1 id="toc0"><span>H</span></h1></blockquote></blockquote>"#,
        ),
        (
            r#"scout-line-owner-block-ul-code-opener-only"#,
            r#"* [[code]]
A
[[/code]]"#,
            r#"<ul>
<li>[[code]]</li>
</ul><p>A<br>
[[/code]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-code-all-marked"#,
            r#"* [[code]]
* A
* [[/code]]"#,
            r#"<ul>
<li>[[code]]</li>
<li>A</li>
<li>[[/code]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-code-opener-close-marked"#,
            r#"* [[code]]
A
* [[/code]]"#,
            r#"<ul>
<li>[[code]]</li>
</ul><p>A</p><ul>
<li>[[/code]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-div-opener-only"#,
            r#"* [[div]]
A
[[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-div-all-marked"#,
            r#"* [[div]]
* A
* [[/div]]"#,
            r#"<ul>
<li>A</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-div-opener-close-marked"#,
            r#"* [[div]]
A
* [[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-collapsible-opener-only"#,
            r#"* [[collapsible]]
A
[[/collapsible]]"#,
            r#"<ul>
<li><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></li>
</ul><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-collapsible-all-marked"#,
            r#"* [[collapsible]]
* A
* [[/collapsible]]"#,
            r#"<ul>
<li><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></li>
</ul><ul>
<li>A</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-collapsible-opener-close-marked"#,
            r#"* [[collapsible]]
A
* [[/collapsible]]"#,
            r#"<ul>
<li><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></li>
</ul><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-css-opener-only"#,
            r#"* [[module CSS]]
x{}
[[/module]]"#,
            r#"<ul>
<li>[[module CSS]]</li>
</ul><p>x{}<br>
[[/module]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-css-all-marked"#,
            r#"* [[module CSS]]
* x{}
* [[/module]]"#,
            r#"<ul>
<li>[[module CSS]]</li>
<li>x{}</li>
<li>[[/module]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-css-opener-close-marked"#,
            r#"* [[module CSS]]
x{}
* [[/module]]"#,
            r#"<ul>
<li>[[module CSS]]</li>
</ul><p>x{}</p><ul>
<li>[[/module]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-unknown-opener-only"#,
            r#"* [[module FooBar]]"#,
            r#"<ul>
<li>[[module FooBar]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-unknown-all-marked"#,
            r#"* [[module FooBar]]"#,
            r#"<ul>
<li>[[module FooBar]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-unknown-opener-close-marked"#,
            r#"* [[module FooBar]]"#,
            r#"<ul>
<li>[[module FooBar]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-align-opener-only"#,
            r#"* [[=]]
A
[[/=]]"#,
            r#"<ul>
<li>[[=]]</li>
</ul><p>A<br>
[[/=]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-ul-align-all-marked"#,
            r#"* [[=]]
* A
* [[/=]]"#,
            r#"<ul>
<li>[[=]]</li>
<li>A</li>
<li>[[/=]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-align-opener-close-marked"#,
            r#"* [[=]]
A
* [[/=]]"#,
            r#"<ul>
<li>[[=]]</li>
</ul><p>A</p><ul>
<li>[[/=]]</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-table-opener-only"#,
            r#"* || A ||"#,
            r#"<ul>
<li>|| A ||</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-table-all-marked"#,
            r#"* || A ||"#,
            r#"<ul>
<li>|| A ||</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-table-opener-close-marked"#,
            r#"* || A ||"#,
            r#"<ul>
<li>|| A ||</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-heading-opener-only"#,
            r#"* + H"#,
            r#"<ul>
<li>+ H</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-heading-all-marked"#,
            r#"* + H"#,
            r#"<ul>
<li>+ H</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ul-heading-opener-close-marked"#,
            r#"* + H"#,
            r#"<ul>
<li>+ H</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-block-ol-code-opener-only"#,
            r#"# [[code]]
A
[[/code]]"#,
            r#"<ol>
<li>[[code]]</li>
</ol><p>A<br>
[[/code]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-code-all-marked"#,
            r#"# [[code]]
# A
# [[/code]]"#,
            r#"<ol>
<li>[[code]]</li>
<li>A</li>
<li>[[/code]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-code-opener-close-marked"#,
            r#"# [[code]]
A
# [[/code]]"#,
            r#"<ol>
<li>[[code]]</li>
</ol><p>A</p><ol>
<li>[[/code]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-div-opener-only"#,
            r#"# [[div]]
A
[[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-div-all-marked"#,
            r#"# [[div]]
# A
# [[/div]]"#,
            r#"<ol>
<li>A</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-div-opener-close-marked"#,
            r#"# [[div]]
A
# [[/div]]"#,
            r#"<p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-collapsible-opener-only"#,
            r#"# [[collapsible]]
A
[[/collapsible]]"#,
            r#"<ol>
<li><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></li>
</ol><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-collapsible-all-marked"#,
            r#"# [[collapsible]]
# A
# [[/collapsible]]"#,
            r#"<ol>
<li><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></li>
</ol><ol>
<li>A</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-collapsible-opener-close-marked"#,
            r#"# [[collapsible]]
A
# [[/collapsible]]"#,
            r#"<ol>
<li><div class="collapsible-block"><div class="collapsible-block-folded"><a class="collapsible-block-link" href="javascript:;">+&nbsp;show&nbsp;block</a></div><div class="collapsible-block-unfolded" style="display:none"><div class="collapsible-block-unfolded-link"><a class="collapsible-block-link" href="javascript:;">–&nbsp;hide&nbsp;block</a></div><div class="collapsible-block-content"></div></div></div></li>
</ol><p>A</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-css-opener-only"#,
            r#"# [[module CSS]]
x{}
[[/module]]"#,
            r#"<ol>
<li>[[module CSS]]</li>
</ol><p>x{}<br>
[[/module]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-css-all-marked"#,
            r#"# [[module CSS]]
# x{}
# [[/module]]"#,
            r#"<ol>
<li>[[module CSS]]</li>
<li>x{}</li>
<li>[[/module]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-css-opener-close-marked"#,
            r#"# [[module CSS]]
x{}
# [[/module]]"#,
            r#"<ol>
<li>[[module CSS]]</li>
</ol><p>x{}</p><ol>
<li>[[/module]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-unknown-opener-only"#,
            r#"# [[module FooBar]]"#,
            r#"<ol>
<li>[[module FooBar]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-unknown-all-marked"#,
            r#"# [[module FooBar]]"#,
            r#"<ol>
<li>[[module FooBar]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-unknown-opener-close-marked"#,
            r#"# [[module FooBar]]"#,
            r#"<ol>
<li>[[module FooBar]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-align-opener-only"#,
            r#"# [[=]]
A
[[/=]]"#,
            r#"<ol>
<li>[[=]]</li>
</ol><p>A<br>
[[/=]]</p>"#,
        ),
        (
            r#"scout-line-owner-block-ol-align-all-marked"#,
            r#"# [[=]]
# A
# [[/=]]"#,
            r#"<ol>
<li>[[=]]</li>
<li>A</li>
<li>[[/=]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-align-opener-close-marked"#,
            r#"# [[=]]
A
# [[/=]]"#,
            r#"<ol>
<li>[[=]]</li>
</ol><p>A</p><ol>
<li>[[/=]]</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-table-opener-only"#,
            r#"# || A ||"#,
            r#"<ol>
<li>|| A ||</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-table-all-marked"#,
            r#"# || A ||"#,
            r#"<ol>
<li>|| A ||</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-table-opener-close-marked"#,
            r#"# || A ||"#,
            r#"<ol>
<li>|| A ||</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-heading-opener-only"#,
            r#"# + H"#,
            r#"<ol>
<li>+ H</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-heading-all-marked"#,
            r#"# + H"#,
            r#"<ol>
<li>+ H</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-block-ol-heading-opener-close-marked"#,
            r#"# + H"#,
            r#"<ol>
<li>+ H</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-transition-same-ul"#,
            r#"> * A
* B"#,
            r#"<blockquote><ul>
<li>A</li>
</ul></blockquote><ul>
<li>B</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-transition-same-ol"#,
            r#"> # A
# B"#,
            r#"<blockquote><ol>
<li>A</li>
</ol></blockquote><ol>
<li>B</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-transition-ul-to-ol"#,
            r#"> * A
# B"#,
            r#"<blockquote><ul>
<li>A</li>
</ul></blockquote><ol>
<li>B</li>
</ol>"#,
        ),
        (
            r#"scout-line-owner-transition-ol-to-ul"#,
            r#"> # A
* B"#,
            r#"<blockquote><ol>
<li>A</li>
</ol></blockquote><ul>
<li>B</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-transition-plain"#,
            r#"> A
B"#,
            r#"<blockquote><p>A</p></blockquote><p>B</p>"#,
        ),
        (
            r#"scout-line-owner-transition-heading"#,
            r#"> + A
+ B"#,
            r#"<blockquote><h1 id="toc0"><span>A</span></h1></blockquote><h1 id="toc1"><span>B</span></h1>"#,
        ),
        (
            r#"scout-line-owner-transition-table"#,
            r#"> || A ||
|| B ||"#,
            r#"<blockquote><table class="wiki-content-table">
<tr>
<td>A</td>
</tr>
</table></blockquote><table class="wiki-content-table">
<tr>
<td>B</td>
</tr>
</table>"#,
        ),
        (
            r#"scout-line-owner-transition-quote"#,
            r#">> A
> B"#,
            r#"<blockquote><blockquote><p>A</p></blockquote><p>B</p></blockquote>"#,
        ),
        (
            r#"scout-line-owner-transition-blank-ul"#,
            r#"> * A

* B"#,
            r#"<blockquote><ul>
<li>A</li>
</ul></blockquote><ul>
<li>B</li>
</ul>"#,
        ),
        (
            r#"scout-line-owner-transition-empty-quote-ul"#,
            r#"> * A
>
* B"#,
            r#"<blockquote><ul>
<li>A</li>
</ul></blockquote><ul>
<li>B</li>
</ul>"#,
        ),
    ] {
        assert_eq!(render(source), expected, "{case_id}: {source:?}");
    }
}

#[test]
fn lost_owner_body_keeps_metadata_and_collapsible_arguments() {
    let html = render(concat!(
        "> [[div]]\n",
        "outside[[footnote]]note[[/footnote]]\n",
        "[[/div]]\n",
        "* [[collapsible folded=\"no\" show=\"OPEN\" hide=\"CLOSE\" hideLocation=\"both\"]]\n",
        "body\n",
        "[[/collapsible]]",
    ));

    assert!(
        html.contains("outside<sup class=\"footnoteref\">"),
        "{html}"
    );
    assert!(html.contains("class=\"footnotes-footer\""), "{html}");
    assert!(html.contains("OPEN"), "{html}");
    assert!(html.contains("CLOSE"), "{html}");
    assert_eq!(
        html.matches("collapsible-block-unfolded-link").count(),
        2,
        "{html}"
    );
    assert!(html.contains("<p>body</p>"), "{html}");
}

#[test]
fn malformed_and_cross_closed_owners_fail_closed() {
    for source in [
        "> [[div]]\nbody",
        "> [[div]]\nbody\n[[/collapsible]]",
        "* [[collapsible]]\n* body\n* [[/div]]",
        "# [[div class=\"unterminated]]\nbody\n[[/div]]",
    ] {
        let html = render(source);
        assert!(!html.is_empty(), "{source:?}");
    }
}

#[test]
fn repeated_lost_owner_sections_stay_bounded() {
    let unit = concat!(
        "> [[div]]\nQ\n[[/div]]\n",
        "* [[collapsible]]\n* L\n* [[/collapsible]]\n",
    );
    let source = unit.repeat(2_048);
    let started = Instant::now();
    let html = render(&source);
    let elapsed = started.elapsed();

    assert_eq!(html.matches("<p>Q</p>").count(), 2_048, "{html}");
    assert_eq!(
        html.matches("collapsible-block-folded").count(),
        2_048,
        "{html}"
    );
    assert_eq!(html.matches("<li>L</li>").count(), 2_048, "{html}");
    assert!(
        elapsed < Duration::from_secs(3),
        "lost-owner parsing took {elapsed:?}",
    );
}
