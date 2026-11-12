# About Folia

Folia is a static site generator written in Rust. I started it to replace Jekyll on my own site,
[ggirelli.info](https://ggirelli.info), and made it usable by others along the way. This page
holds the full story: where it comes from, how it is made, and what that means for you.

## Where it comes from

My site had grown a blog, short notes, likes and bookmarks, and a digital garden, and Jekyll built
them with a stack of plugins and a build I waited for. I wanted three things: builds fast enough
to stay out of the way, one coherent workflow for every collection of content, and a site that is plain
files I own. Folia is that, built around the content I actually publish. It is not a faster
Jekyll: it has its own configuration, its own templates, and a fixed set of collections.

## Fully vibe coded

!!! warning "Fully vibe coded"
    Folia is written entirely by an AI coding assistant: Claude, made by Anthropic, writes the
    code, tests, scripts, documentation and plans in conversation with the project's maintainer,
    who describes what they want, reviews the results and asks for changes. No line of it is
    written by hand.

- **It is tested, not audited.** Changes are checked with automated checks and against production
  builds of the Jekyll site, but nobody reviews the code line by line.
- **Expect rough edges.** Please [report bugs](https://github.com/ggirelli/folia-web/issues).
- **It was started for one website**, and makes no promise to suit yours.
- **No warranty.** The software is provided "as is" under the MIT license.

I say it because it is true, and because you should know how the software you run was made.

## The process

Design comes first: each architecture proposal was a versioned plan, revised until the maintainer
approved it. Risky decisions were then settled by small validation spikes, each with explicit
success criteria checked before moving on. Every commit is co-authored by the assistant, so the
history shows the whole process, and the plans are kept in it.

The same text is in
[DISCLAIMER.md](https://github.com/ggirelli/folia-web/blob/main/DISCLAIMER.md) in the repository.
To see how the code is organized, read [Architecture](internals/architecture.md).
