# How it is made

!!! warning "Fully vibe coded"
    Folia is being written entirely by an AI coding assistant: Claude, made by Anthropic, writes
    the code, tests, scripts, documentation and plans in conversation with the project's
    maintainer, who describes what they want, reviews the results and asks for changes. No line of
    it is written by hand.

- **It is tested, not audited:** changes are checked with automated checks and against production
  builds of the Jekyll site, but nobody reviews the code line by line.
- **Expect rough edges:** please [report bugs](https://github.com/ggirelli/folia-web/issues).
- **It is built for one website** and makes no promise to suit yours.
- **No warranty:** the software is provided "as is" under the MIT license.

The full text is in
[DISCLAIMER.md](https://github.com/ggirelli/folia-web/blob/main/DISCLAIMER.md).

## The process

Design comes first: each architecture proposal was a versioned plan, revised until the maintainer
approved it. Risky decisions were then settled by small validation spikes, each with explicit
success criteria checked before moving on. Every commit is co-authored by the assistant, so the
history shows the whole process, and the plans are kept in it.
