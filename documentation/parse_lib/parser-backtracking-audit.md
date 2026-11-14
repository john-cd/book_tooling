<!-- TODO Reviewed ??-??-2026. -->

# Parser backtracking and recursion audit

Audit date: 2026-11-05. Scope: the `winnow` parsers in `parse_lib/src` (`markdown`, `parse_parts`,
`parse_directives`, `parse_wikilinks`, `parse_urls`, `parse_hidden`, `parse_document`).

## Method

A throwaway test generated hostile documents that repeat one unmatched or unbalanced construct
(`[`, `(`, `{{`, `[a](`, `<`, a backtick, `![`, `[[ `, `| a `, `#`, `*`, `[a]: `, `[a][`,
`{{#crate a `, a backslash, `<div class="hidden">`, a code fence, `<http://`, `[a](<`, `![a](`,
`{{!`, `{{#example `, `[[a|`, `&`, `_a`, `a_`) 1,500 and 3,000 times. It timed `parse_document`
plus rendering in a release build and compared the two sizes. It also nested brackets and
parentheses 100,000 deep in unit tests.

## Findings

- **No exponential backtracking.** Doubling the input multiplied the time by 1.1 to 4.0 for every
  construct, which is linear to mildly quadratic growth.
- **Quadratic cost for unterminated openers.** Each unmatched opener rescans the rest of the input
  before the parser falls back to plain text. This is inherent in trying every position with
  several alternatives. The worst measured case is 3,000 repetitions of `{{#crate a ` (33 KB) at
  about 1.3 s; `<div class="hidden">` took about 0.4 s and `{{#example ` about 0.35 s for the same
  count. Realistic books are far below these sizes.
- **Unbounded recursion was a real defect, now fixed.** Link text (`[` nested in `[`) and link
  destinations (`(` nested in `(`) recursed once per nesting level, so a few thousand unmatched
  brackets overflowed the stack (a process abort, not an error). Both now scan depth with a loop,
  and unit tests cover 100,000 levels.
- **No other unbounded recursion was found.** Blockquotes hold recursively parsed elements, but
  20,000 nested `> ` markers parsed in under a millisecond without overflowing the stack; the
  remaining combinators are bounded repetitions over a flat alternative list.

## Remaining risk and guidance

- The quadratic cost is accepted. If a future workload needs more headroom, cache the "no closing
  delimiter" result per opener (for example the position of the last `}}` or `-->`) so repeated
  failures stop rescanning.
- Keep new nested constructs iterative, and add a deep-nesting test next to them.
- `parse_lib/fuzz` (`round_trip`) exercises the parsers continuously; see the workflow in
  `.github/workflows/fuzz.yml`.
