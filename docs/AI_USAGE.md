# AI usage

This project was developed with AI coding agents (Claude Code, running
Anthropic Claude models) working alongside a human author, under continuous
human review.

- [`AGENTS.md`](../AGENTS.md) and
  [`_dev/dev_AGENTS.md`](../_dev/dev_AGENTS.md) are the operating protocols
  those agents follow: how to verify a claim before stating it, when to stop
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
