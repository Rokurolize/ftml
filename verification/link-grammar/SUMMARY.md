# Track T5 Link Grammar Differential & Adjudication Summary

Total generated cases: **1688**

## Raw Differential Counts

- `disagree`: 559
- `disagree_save_inadmissible`: 144
- `equal`: 985

## Adjudication Breakdown

- `ftml_bug_candidate`: 64
- `intentional_security_sanitization`: 4
- `known_fixed_in_flight_862`: 49
- `known_separate_family_18081`: 124
- `out_of_scope_save_inadmissible`: 144
- `pinned_benign_format`: 278
- `pinned_equal`: 985
- `ref_or_undecided_bracket_interior`: 40

## Findings 854-859, 862 Coverage

- Finding 854: 165 generated rows (e.g. t5-0031:pinned_equal, t5-0032:pinned_equal, t5-0067:pinned_equal, t5-0068:pinned_equal, t5-0103:pinned_equal)
- Finding 855: 99 generated rows (e.g. t5-0577:pinned_equal, t5-0578:pinned_equal, t5-0579:pinned_equal, t5-0584:pinned_equal, t5-0585:pinned_equal)
- Finding 856: 84 generated rows (e.g. t5-1078:pinned_benign_format, t5-1079:pinned_benign_format, t5-1080:pinned_benign_format, t5-1081:pinned_benign_format, t5-1082:pinned_benign_format)
- Finding 857: 96 generated rows (e.g. t5-0003:pinned_equal, t5-0004:pinned_equal, t5-0005:pinned_equal, t5-0006:pinned_equal, t5-0007:pinned_equal)
- Finding 858: 160 generated rows (e.g. t5-0009:pinned_benign_format, t5-0010:pinned_benign_format, t5-0011:pinned_equal, t5-0012:pinned_equal, t5-0013:pinned_equal)
- Finding 859: 160 generated rows (e.g. t5-0015:pinned_equal, t5-0016:out_of_scope_save_inadmissible, t5-0017:pinned_equal, t5-0018:out_of_scope_save_inadmissible, t5-0019:pinned_equal)
- Finding 862: 230 generated rows (e.g. t5-0805:known_fixed_in_flight_862, t5-0810:known_fixed_in_flight_862, t5-0817:pinned_equal, t5-0818:known_fixed_in_flight_862, t5-0819:pinned_equal)

## Actionable & Undecided Clusters

### ftml_bug_candidate:double_pipe_bracket_fallback_truncation (22 rows)

- `t5-0607` source='[http://ab||-\xa0label]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p>[<a href="http://ab||-%C2%A0label">http://ab||- label</a>]</p>`
  - ftml: `<p>[<a href="http://ab">http://ab</a>||-&#160;label]</p>`
- `t5-0611` source='[http://ab||\xa0label]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p>[<a href="http://ab||%C2%A0label">http://ab|| label</a>]</p>`
  - ftml: `<p>[<a href="http://ab">http://ab</a>||&#160;label]</p>`
- `t5-0690` source='[*http://ab||-\xa0label]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p>[<a href="http://ab||-%C2%A0label" target="_blank">http://ab||- label</a>]</p>`
  - ftml: `<p>[<a href="http://ab" target="_blank">http://ab</a>||-&#160;label]</p>`
- `t5-0694` source='[*http://ab||\xa0label]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p>[<a href="http://ab||%C2%A0label" target="_blank">http://ab|| label</a>]</p>`
  - ftml: `<p>[<a href="http://ab" target="_blank">http://ab</a>||&#160;label]</p>`
- `t5-0764` source='[[[http://ab||c]]]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p><a href="http://ab">|c</a></p>`
  - ftml: `<p><a href="http://ab||c">http://ab||c</a></p>`
- `t5-0765` source='[[[http://ab||c|label]]]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p><a href="http://ab">|c|label</a></p>`
  - ftml: `<p><a href="http://ab||c">label</a></p>`
- `t5-0766` source='[[[http://ab||-]]]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p><a href="http://ab">|-</a></p>`
  - ftml: `<p><a href="http://ab||-">http://ab||-</a></p>`
- `t5-0767` source='[[[http://ab||-|label]]]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p><a href="http://ab">|-|label</a></p>`
  - ftml: `<p><a href="http://ab||-">label</a></p>`
- `t5-0768` source='[[[http://ab||]]]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p><a href="http://ab">|</a></p>`
  - ftml: `<p><a href="http://ab||">http://ab||</a></p>`
- `t5-0769` source='[[[http://ab|||label]]]' (ftml url.rs:400-406,644 bracket_fallback only set after ]])
  - ref: `<p><a href="http://ab">||label</a></p>`
  - ftml: `<p><a href="http://ab||">label</a></p>`
- ... and 12 more

### ftml_bug_candidate:freelink_star_path_label (1 rows)

- `t5-1683` source='[[[*/path]]]' (Freelink.php:195 strips * from no-label /path display text)
  - ref: `<p><a target="_blank" href="/path">/path</a></p>`
  - ftml: `<p><a href="/path" target="_blank">*/path</a></p>`

### ftml_bug_candidate:pipe_backslash_href_slash (5 rows)

- `t5-0581` source='http://ab|\\c' (ftml normalizes backslash to / in href whereas PHP ref keeps \)
  - ref: `<p><a href="http://ab|\c">http://ab|\c</a></p>`
  - ftml: `<p><a href="http://ab|/c">http://ab|\c</a></p>`
- `t5-0616` source='[http://ab|\\c]' (ftml normalizes backslash to / in href whereas PHP ref keeps \)
  - ref: `<p>[<a href="http://ab|\c">http://ab|\c</a>]</p>`
  - ftml: `<p>[<a href="http://ab|/c">http://ab|\c</a>]</p>`
- `t5-0619` source='[http://ab|\\c\xa0label]' (ftml normalizes backslash to / in href whereas PHP ref keeps \)
  - ref: `<p>[<a href="http://ab|\c%C2%A0label">http://ab|\c label</a>]</p>`
  - ftml: `<p>[<a href="http://ab|/c%C2%A0label">http://ab|\c&#160;label</a>]</p>`
- `t5-0699` source='[*http://ab|\\c]' (ftml normalizes backslash to / in href whereas PHP ref keeps \)
  - ref: `<p>[<a href="http://ab|\c" target="_blank">http://ab|\c</a>]</p>`
  - ftml: `<p>[<a href="http://ab|/c" target="_blank">http://ab|\c</a>]</p>`
- `t5-0702` source='[*http://ab|\\c\xa0label]' (ftml normalizes backslash to / in href whereas PHP ref keeps \)
  - ref: `<p>[<a href="http://ab|\c%C2%A0label" target="_blank">http://ab|\c label</a>]</p>`
  - ftml: `<p>[<a href="http://ab|/c%C2%A0label" target="_blank">http://ab|\c&#160;label</a>]</p>`

### ftml_bug_candidate:sentence_boundary_right_link (1 rows)

- `t5-0906` source='[*http://a.]]]' (ftml url.rs:465-472 at_sentence_boundary misses Token::RightLink)
  - ref: `<p>[<a href="http://a" target="_blank">http://a</a>.]]]</p>`
  - ftml: `<p>[<a href="http://a." target="_blank">http://a.</a>]]]</p>`

### ftml_bug_candidate:unlabeled_interwiki_right_block (1 rows)

- `t5-1505` source='[wikipedia:Page]]' (ftml link_single.rs:429 closes=[RightBracket] misses RightBlock)
  - ref: `<p><a href="https://en.wikipedia.org/wiki/Page" onclick="window.open(this.href, '_blank'); return false;">Page</a>]</p>`
  - ftml: `<p>[wikipedia:Page]]</p>`

### ftml_bug_candidate:wikidot_only_scheme_empty_host (34 rows)

- `t5-0039` source='ssh://[' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>ssh://[</p>`
  - ftml: `<p><a href="ssh://[">ssh://[</a></p>`
- `t5-0041` source='ssh://]' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>ssh://]</p>`
  - ftml: `<p><a href="ssh://">ssh://</a>]</p>`
- `t5-0043` source='ssh://|' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>ssh://|</p>`
  - ftml: `<p><a href="ssh://">ssh://</a>|</p>`
- `t5-0045` source='ssh://\xa0' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>ssh:// </p>`
  - ftml: `<p><a href="ssh://">ssh://</a>&#160;</p>`
- `t5-0047` source='ssh://\x85' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>ssh://</p>`
  - ftml: `<p><a href="ssh://">ssh://</a></p>`
- `t5-0049` source='ssh://\u3000' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>ssh://　</p>`
  - ftml: `<p><a href="ssh://">ssh://</a>　</p>`
- `t5-0111` source='*ssh://[' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>*ssh://[</p>`
  - ftml: `<p><a href="ssh://[" target="_blank">ssh://[</a></p>`
- `t5-0113` source='*ssh://]' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>*ssh://]</p>`
  - ftml: `<p><a href="ssh://" target="_blank">ssh://</a>]</p>`
- `t5-0115` source='*ssh://|' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>*ssh://|</p>`
  - ftml: `<p><a href="ssh://" target="_blank">ssh://</a>|</p>`
- `t5-0117` source='*ssh://\xa0' (ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS)
  - ref: `<p>*ssh:// </p>`
  - ftml: `<p><a href="ssh://" target="_blank">ssh://</a>&#160;</p>`
- ... and 24 more

### ref_or_undecided_bracket_interior:bracket_interior_or_space_anchor_link (5 rows)

- `t5-1378` source='[[# label]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[<a href="javascript:;">label</a></p>`
  - ftml: `<p>[[# label]</p>`
- `t5-1453` source='[[[# ]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a class="newpage" href="/#"></a></p>`
  - ftml: `<p>[[[# ]]]</p>`
- `t5-1454` source='[[[# label]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a class="newpage" href="/# label"></a></p>`
  - ftml: `<p>[<a name="label"></a>]</p>`
- `t5-1456` source='[[[#\t]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a class="newpage" href="/#"></a></p>`
  - ftml: `<p>[[[# ]]]</p>`
- `t5-1463` source='[[[#]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a class="newpage" href="/#"></a></p>`
  - ftml: `<p>[[[#]]]</p>`

### ref_or_undecided_bracket_interior:bracket_interior_or_space_scheme_url (35 rows)

- `t5-0435` source='[[[http://[]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[http://[]]]</p>`
  - ftml: `<p><a href="http://[">http://[</a></p>`
- `t5-0436` source='[[[http://[b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[<a href="http://[b">http://[b</a>]]]</p>`
  - ftml: `<p><a href="http://[b">http://[b</a></p>`
- `t5-0438` source='[[[http://]b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[<a href="http://]b">http://]b</a>]]]</p>`
  - ftml: `<p><a href="http://]b">http://]b</a></p>`
- `t5-0440` source='[[[http://|b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a href="http://">b</a></p>`
  - ftml: `<p><a href="http://|b">http://|b</a></p>`
- `t5-0468` source='[[[http:// b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a href="http:// b">http:// b</a></p>`
  - ftml: `<p><a href="http://%20b">http:// b</a></p>`
- `t5-0472` source='[[[ssh://[b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[<a href="ssh://[b">ssh://[b</a>]]]</p>`
  - ftml: `<p><a href="ssh://[b">ssh://[b</a></p>`
- `t5-0474` source='[[[ssh://]b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[<a href="ssh://]b">ssh://]b</a>]]]</p>`
  - ftml: `<p><a href="ssh://]b">ssh://]b</a></p>`
- `t5-0504` source='[[[ssh:// b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p><a href="ssh:// b">ssh:// b</a></p>`
  - ftml: `<p><a href="ssh://%20b">ssh:// b</a></p>`
- `t5-0507` source='[[[*http://[]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[*http://[]]]</p>`
  - ftml: `<p><a href="http://[" target="_blank">http://[</a></p>`
- `t5-0508` source='[[[*http://[b]]]' (Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs)
  - ref: `<p>[[[<a href="http://[b" target="_blank">http://[b</a>]]]</p>`
  - ftml: `<p><a href="http://[b" target="_blank">http://[b</a></p>`
- ... and 25 more

