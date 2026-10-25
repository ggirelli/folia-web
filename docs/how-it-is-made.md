# How it is made

!!! warning "Fully vibe coded"
    Folia is being written entirely by an AI coding assistant: Claude, made by Anthropic, writes
    the code, tests, scripts, documentation and plans in conversation with the project's
    maintainer, who describes what they want, reviews the results and asks for changes. No line of
    it is written by hand.

- **It is tested, not audited:** changes are checked with automated checks and against production
  builds of the Jekyll site, but nobody reviews the code line by line.
- **Expect rough edges:** please [report bugs](https://github.com/ggirelli/folia/issues).
- **It is built for one website** and makes no promise to suit yours.
- **No warranty:** the software is provided "as is" under the MIT license.

The full text is in
[DISCLAIMER.md](https://github.com/ggirelli/folia/blob/main/DISCLAIMER.md).

## The process

Design comes first. Each architecture proposal is a versioned plan in [planning](planning/index.md),
revised until the maintainer approves it. Risky decisions are then settled by small validation
spikes, each with explicit success criteria that are checked before moving on. Every commit is
co-authored by the assistant, so the history shows the whole process.
