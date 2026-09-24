# AI usage

This project was developed with AI coding agents working alongside a human
author, under continuous human review. Three AI tools were used:

- **Cline** (VS Code extension), running Anthropic Claude models: Opus for
  planning and Sonnet for implementation. Used from the start of development
  (2026-07-06) until 2026-07-15.
- **Claude Code**, running Anthropic Claude models. Used from 2026-07-15 for
  most of the remaining design, code, tests, and documentation.
- **Perplexity**, in three roles:
  - as a coding-session host running Claude Sonnet and Opus, on 2026-08-19,
    when API access was unavailable;
  - as an adversarial reviewer in design discussions with the human author.
    It challenged the benchmark comparison (2026-08-17). It also took part in
    the back-and-forth on how to decide whether a modification is real when
    Sage gives only a precursor mass plus a delta mass, with no localization
    score. The adopted design, a small curated modification list tested
    with Fisher's exact test and an odds-ratio floor, came out of that
    discussion;
  - with Sonar, to research and draft several of the distilled notes in
    [`_dev/reference-notes/`](../_dev/reference-notes/). Those notes are
    secondary summaries. The process is described in
  [`_dev/reference-notes/VENDOR-CHECKLIST.md`](../_dev/reference-notes/VENDOR-CHECKLIST.md).
  Their links are pointers to sources, not citations, and some still carry
  unresolved citation placeholders.

Beyond the tools above:

- [`AGENTS.md`](../AGENTS.md) and
  [`_dev/dev_AGENTS.md`](../_dev/dev_AGENTS.md) are the operating protocols
  the coding agents follow: how to verify a claim before stating it, when to stop
  and flag a contradiction, and what parts of the project are settled and
  should not be reopened.
- Every commit is authored, reviewed, and pushed by Benjamin A. Neely, who is
  accountable for the code, the numbers it produces, and the claims made
  about it. No commit carries an AI co-author trailer.
- AI assistance covered design discussion, code, tests, and documentation
  (including this file). It did not cover independent verification: numerical
  claims are checked against the pipeline's own scripts before they are
  trusted, per the tripwires in `_dev/dev_AGENTS.md`.

This is a plain factual record, not a policy statement. NIST and GitHub do
not yet have a standard disclosure format for AI-assisted development, so
this file states what was actually done rather than following a template.
