# Triple-nested inline crossing inversion — the live Wikidot rule

## STATUS

- **Done — rule found.** The inversion comes from Wikidot's two-stage
  pipeline: Text_Wiki regex passes run in rule order and produce a flat tag
  stream that can be misnested. Then libtidy repairs it in
  `WikiTransformation::purifyHtml`. Tidy's inline-stack (`istack`) mechanics
  produce the strong/em inversion and keep `##` inside the strike span. This
  is not a token-pairing rule, which is why the greedy, lazy and offset
  pairing models all failed.
- **Done — offline simulator.** `misc/wikidot-inline-sim/sim.php` applies the
  inline Text_Wiki regexes (copied from `gabrys/wikidot`), renders the
  tokens, and runs PHP `tidy_parse_string` with Wikidot's config. It needs
  `php8.3-tidy` (local libtidy 5.6.0). Its output matched live PagePreview
  byte-for-byte on **63/63** probes (65 captured in total). It also matched
  75/81 existing corpus `wikidot.html` pins that use only inline syntax. The
  6 misses use URL, table and blockquote rules, which the simulator does not
  model.
- **Done — narrow ftml fix.** `repair_deferred_strike_double_shell_pass`
  (`src/parsing/inline_format.rs`) lowers
  `--a##red|xb{S1}{S2}--c##--{S2}{S1}##` like live: the later-opened shell is
  outermost and holds the dash, the color closes after it, and the strike
  close moves to the end of the sequence so trailing `##` stays inside it.
  It covers every ordered pair of `** // {{ ,,` (12 shapes) and trailing text
  after `##`. Pinned by corpus fixture
  `test/strikethrough/deep-nesting-double-shell-inversion` (live
  `wikidot.html` from PagePreview, 5 inputs). The full `cargo test` passes
  (1160 lib, 582 security, doc test) and `cargo fmt` is clean. The only
  clippy errors are pre-existing lints outside this change.
- **Partial — ftml overall.** Before the fix ftml matched live on 8/63 of the
  simulator-checked probes; after it, 23/63. The other 40 are different
  shapes that need the faithful istack lowering below (underline pairs,
  plain crossings like `**a//b--c**d--e//`, `--a**//--c//**` without a color,
  `##red|a**//b##c//**##`, and shapes with text between the content and the
  shell closes).

## Pipeline (source: gabrys/wikidot)

1. Inline rules run in the order `Colortext, Strong, Emphasis, Underline,
   Strikethrough, Tt, Superscript, Subscript, Typography`. Each pass runs
   `preg_replace_callback` over the whole source and replaces each match
   with `\xFF<id>\xFF` start/end tokens.
   - The regexes are lazy (`/U`): Strong `/\*\*([^\s\n](?:.*[^\s\n])?)\*\*/U`,
     Emphasis `/\/\/([^\s](?:.*[^\s])?)\/\//U`, Strike
     `/\-\-([^\s](?:.*[^\s])?)\-\-/U`. Colortext is
     `/\#\#(.+?)\|(.+?)\#\#/`. Typography turns each remaining `--` into an
     em dash.
   - Token bytes count as non-space, so a later rule pairs across the tokens
     of earlier rules.
   - Strikethrough, underline and color all render as `<span>`.
2. libtidy: `tidy_parse_string(..., ['indent'=>false,'output-xhtml'=>true,
   'wrap'=>0])`. The relevant code is `ParseInline` in `parser.c` and
   `istack.c` in tidy-html5 5.6.0.
   - `PushInline` skips implicit nodes, and skips a tag that is already on
     the istack. So a second `<span>` is never pushed while one is open.
   - When an end tag matches the current element, it closes the element and
     `PopInline` pops the **top** of the istack, which may not be that
     element's entry.
   - When an inline end tag `</X>` arrives inside inline element E, X≠E, and
     both are pushed, tidy calls `SwitchInline(E,X)`. This swaps the entry
     for E (the topmost one) with the nearest entry for X below it. The end
     tag is pushed back, E closes **without** being popped, and `InlineDup1`
     sets the insert pointer at E's istack entry. Implicit copies of the
     istack, from that entry up to the top, then reopen before the next
     *text or start* token. A pushed-back end tag is processed first.
   - If the swap fails, or X is not pushed, tidy pops the istack top, closes
     E and **discards** `</X>`.
   - Empty inline elements are dropped when the tree is cleaned up.

## Worked trace: `--a##red|xb**//--c##--//**##`

The Text_Wiki stage produces (`s` = strike span, `C` = color span):

    <s>a<C>xb<strong><em></s>c</C>—</em></strong>##

Tidy then processes it as follows:

1. The istack is `[span(s), strong, em]`. `C` is not pushed because a span
   is already on the stack.
2. The first `</span>` arrives inside `em`. `SwitchInline` swaps the stack to
   `[em, strong, span]`, the end tag is pushed back, `em` closes (empty) and
   the insert pointer is set to index 0.
3. `</span>` inside `strong`: no swap is possible, so tidy pops the top
   (`span`), discards the end tag and closes `strong` (empty).
4. Back inside `C`, the text `c` triggers implicit reopens from the istack:
   `<em><strong>`. This is the inversion.
5. `</C>` arrives inside `strong`. Span is no longer pushed, so tidy pops
   `strong` and discards the end tag. `—` then lands inside `em`.
6. `</em>` closes `em`. `</strong>` then closes `C` (non-matching end tag),
   and `##` stays inside `s`.

Result, identical to live:

    <s>a<C>xb<em><strong>c</strong>—</em></C>##</s>

The same mechanics, applied to the sibling with the shells swapped
(`//**`), give `<strong><em>c</em>—</strong>`. So it is the *later-opened*
shell that ends up outermost; em is not always nested inside strong.

With a single shell (the pinned D1 case), the same trace gives
`<s>a<C>xb</C><X>c</X>—</s>##`.

## Evidence

All captures were anonymous POSTs of `moduleName=edit/PagePreviewModule` to
`https://scp-sandbox-3.wikidot.com/ajax-module-connector.php`. The probes
covered:

- every ordered pair of `** // __ {{ ,,` inside
  `--a##red|xb{S1}{S2}--c##--{S2}{S1}##`;
- six three-shell variants;
- for `**`/`//` in both orders: shifted text, extra trailing text, missing
  or extra `##`/`--`, no color, no strike, and plain crossings
  (`**a//b**c//`, `**a//b--c**d--e//`);
- a nested blue color (`##blue|…`) and crossings of color with bold/italic.

The simulator matched all 63 probes. Some cases where ftml diverges:

| source | live (= simulator) | ftml |
|---|---|---|
| `--a**//--c//**` | `<s>a<em><strong>c</strong></em></s>` | `<s>a</s><strong><em>c</em></strong>` |
| `##red\|a**//b##c//**##` | `<C>a<strong><em>b</em></strong><em><strong>c</strong></em>##</C>` | `<C>a<strong><em>b##c</em></strong></C>` |
| `**a//b--c**d--e//` | `<strong>a<em>b<s>c</s></em><s><em>d</em></s><em>e</em></strong>` | `**a<em>b<s>c**d</s>e</em>` |

The last row shows that ftml's stage-1 pairing also diverges: Strong pairs
first, across later rules.

## Why ftml misses it

The D1 repair pass (`repair_deferred_strike_trailing_dash_pass` in
`src/parsing/inline_format.rs`) sees a semi-normalized stream, not the raw
Text_Wiki stream. For the two-shell case that stream is:

    [sO a colorO xb BoldO sC ItalO c ItalC BoldC colorC "--" "##"]

The shell closes already come before the color close, so the pattern
expects a single shell and does not match. In addition, `tidy_close` in
`src/parsing/inline_scope.rs` differs from libtidy in two ways:

- it pops the element's entry instead of the istack top;
- after a swap it does not keep the closed element's entry or reopen
  lazily at the next text or start token.

## Remaining work

1. **Faithful lowering (recommended, not done).** Replace or augment
   `tidy_close` in `src/parsing/inline_scope.rs` with an exact
   `ParseInline`/istack emulator: pop the istack top, keep the entry after a
   swap, use an insert pointer, discard unmatched end tags, drop empty
   elements. It must be fed a stream that keeps the positions Text_Wiki
   produces, including stage-1 pairing across earlier rules' tokens. Use
   `misc/wikidot-inline-sim/sim.php` as a differential oracle.
2. Underline (`__`, a span) pairs are not covered by the narrow pass.
3. The probe and compare harness and the 65 live captures were not
   committed; they lived in the session scratchpad.
