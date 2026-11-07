# Folia --- Project Plan (v13)

**Status:** Planning draft, revision 13 (supersedes `plan.v12.md`)\
**Project type:** Personal, in-house static-site publishing engine\
**Implementation language:** Rust\
**Primary reference site:** `ggirelli.info`, Jekyll source in the `filopoe` repository\
**Repository:** `https://github.com/ggirelli/folia-web.git`

Markers, status words and the three operations (push, deploy, publish) are v12's, unchanged.
**Nothing from the migration may be published or deployed until the migration is finalized and
explicitly approved**, as in v12. A push still needs its own authorization (Q19).

## 0. How to read v13

v13 records one change: **Folia's public repository moved to `ggirelli/folia-web`**, with a
sanitized history. It also records the rule that separates the public, generic tooling from a
site's private inputs and evidence.

**It changes no architectural decision, no phase or its order, and no acceptance criterion.**
v12 stays as written, as history.

- **Carried forward from v12 by reference, current and not restated:** §1 to §11 (status,
  evidence, architecture, contracts, parity, roadmap, cutover, acceptance criteria F1–F16, risks,
  decision log, non-goals), except where v13 says so below.
- References of the form *plan v12, §n* in the code and documentation stay valid. New text cites
  v13 for the repository and the public/private separation, and v12 for everything else.

Changes since v12:

- §1: the repository replacement, its dates, and what did not change.
- §2: the separation of public tooling from private site inputs and evidence [DECIDED, Q27].
- §3: the A2 work items delivered since v12's observation of 2026-11-03.
- §4: v12's statements that the replacement has made stale.
- §5: decisions Q27 and Q28 (decided) and Q29 (open).

## 1. The repository replacement

### 1.1 What changed

| Date | Event |
|-|-|
| 2026-11-06 | Folia's history was sanitized: site data was removed from every commit, and every commit was re-signed (§1.3) |
| 2026-11-06 | The original repository, `ggirelli/folia`, was made **private**, then **archived**. Its documentation site stopped being served |
| 2026-11-06 | `ggirelli/folia-web` was created, public, and the sanitized history (`2d1b93d`) was pushed as its `main` |
| 2026-11-07 | Folia's own URLs were updated to the new address (`09f2116`), and the documentation deployed to <https://ggirelli.github.io/folia-web/> |
| 2026-11-07 | The replacement was **accepted** by the project owner (§1.4) |

The repository has the same settings as before: squash-only merging, the "Protect main" ruleset
(pull request, required checks, linear history, signed commits, no deletion or force push),
Dependabot alerts and security updates, secret scanning with push protection, and Pages deployed
by the Docs workflow from `main` only.

### 1.2 What did not change

- **Folia's identity:** the project, the crates (`folia`, `folia-core`, the modes, `parity`), the
  `folia` binary, the `FOLIA_*` environment variables and the branding all stay **Folia**. Only
  the repository's name changed, to `folia-web`.
- The architecture, roadmap and acceptance criteria of v12.
- The reference site, its repository (`filopoe`) and its production deployment. The replacement
  touched none of them.

### 1.3 The sanitized history

`folia-web`'s history is the original history with the site's private data removed. It has
30 commits, from `cb3b7d3` to `2d1b93d`, plus the URL update:

- Files that held the site's data (configuration, ledgers, toolchain records, evidence) were
  removed from every commit. Mentions of private paths, commits and runs in files and commit
  messages were replaced by neutral placeholders (as in v12, e.g. `BASELINE-B`, `SITE-PR`).
- Commits that only added evidence were dropped.
- Every commit was re-signed by the owner, keeping its author and dates, and GitHub verifies
  every one.
- The last commit, `2d1b93d`, moves the site's data out of the tree (§2).

Two consequences for readers of v0–v12:

- **Commit IDs cited in v0–v12 name the original history.** They do not resolve in `folia-web`.
  The owner keeps the mapping from original to sanitized commits privately.
- **Pull request numbers in commit messages** (`(#2)` … `(#12)`) name the original repository's
  pull requests. On `folia-web`, GitHub links them to unrelated pull requests with the same
  numbers.

### 1.4 Acceptance and retention

- **Accepted:** 2026-11-07, by the project owner, on the evidence of the new repository's checks,
  signatures, history and deployment.
- **Transition period:** a defined soak and retention period begins on the acceptance date.
  During it, the new repository and its documentation stay operational, with no regression.
- **The old repository** stays **private and archived** throughout that period. It is deleted
  only after the period ends, and only with the owner's explicit approval at that time. It is
  never made public again.
- The owner keeps the original history and its records privately and permanently. They are not
  published, and no public build depends on them.

## 2. Public tooling and private site inputs [DECIDED, Q27]

Folia's repository holds what any site could use. What belongs to one site is kept by that site's
owner, outside the repository.

| Public, in `folia-web` | Private, kept by the site's owner |
|-|-|
| the generator and its crates | the site's Folia configuration (`folia.toml`, with the slice's documents) |
| the generic parity tools (`crates/parity`, `parity/*.py`, `parity/jekyll/`, `scripts/`) | the deviation ledger and the expected warnings |
| the generic negative checks, with synthetic fixtures | the site's own negative checks and the tests that read its ledgers |
| the shared site exclusions (`parity/site-exclusions.txt`) | the toolchain records, lockfiles and environments, and the pinned times |
| the documentation and the plans | the reference build's recorded manifest (A0) and the baselines |
| CI, Docs and their configuration | **all evidence**: spike, step and checkpoint folders, acceptance runs, kept builds' records |

Rules:

1. **The public build needs nothing private.** `just check`, CI and the Docs deployment run
   with every `FOLIA_*` variable unset.
2. **A recipe that needs a site input names it by environment variable** (`docs/development.md`,
   "A site's private inputs"), and refuses to run without it. It never falls back to a path in
   the repository.
3. **Nothing private is committed to `folia-web`.** `.gitignore` keeps the site's inputs and
   evidence out. A change that would add one is refused in review.
4. **Evidence stays private.** Public documentation may describe what a check does and what kind of
   evidence it writes, but never copy a site's evidence, paths, commits or runs.
5. **No public archive of private material.** The private archive is not a dependency, a
   submodule, a release asset or a remote of `folia-web`.

## 3. Status since v12

v12 observed the repository on 2026-11-03, with A2's work items A2.5 to A2.10 *remaining*. They
have since been built, reviewed and merged. Their evidence is kept privately (§2), and each
work item's acceptance against v12's §6 line is still judged there:

| Work item | Merged as (sanitized history) |
|-|-|
| A2.1 coverage classes, A2.3 closure, Q26 provenance check | `f9fbf84` |
| A2.6 baseline verification | `98fa930` |
| A2.5 determinism | `c23f6c4` |
| A2.7 toolchain comparison | `b17755d`, follow-ups `aa719da` |
| A2.8 route-level links | `dc176d1` |
| A2.9 expected warnings (W-001: Q3l, preserved) | `a1eb9e9` |
| A2.10 acceptance command skeleton (`just accept`) | `d8ca959` |

**Still open in A2, unchanged from v12:**

- A2.3's per-gate messages for `markdown-negative.sh`;
- route parity's *claimed* class and `missing_templates` (v12 §3.4);
- F8's enforced Rouge floor and F14's viewports;
- v11's corrections scheduled "in A2";
- Q3a, Q20, Q22 and Q23;
- the step's acceptance line.

A1 and A3 have not started. Final acceptance (v12 §8) is not reached.

## 4. Stale statements in v12, reconciled

| Statement | Where (v12) | Actual state |
|-|-|-|
| Repository `https://github.com/ggirelli/folia.git`, Jujutsu colocated with Git | header | `https://github.com/ggirelli/folia-web.git` (§1). The original repository is private and archived |
| Commit IDs of the repository state | §1.1, §1.2 | They name the original history (§1.3) |
| Evidence cited by path under `parity/evidence/` | §0, §2, §5.4 | Those folders are kept privately, with the site's inputs (§2). The paths name them there, not in `folia-web` |
| Evidence "committed on its own, with that revision as its only parent" | §5.4, §10.1 (Q26) | That part of Q26 assumed evidence in Folia's repository; where it is now committed is Q29 (§5). Q26's other rules stand: a clean, committed Folia revision, the builds' records kept, and paths as diagnostics |
| Q19(1) and (2): move `main` and push the planning commits | §10.2 | Overtaken: `main` carries step 3, the plans and A2's work |
| Folia's docs published at `ggirelli.github.io/folia/` | §0 (via `docs.yml`) | Published at <https://ggirelli.github.io/folia-web/>. The old address no longer serves |

`docs/baseline.md` (on `just accept`) still describes committing an evidence folder by Q26 as the
commit's only child. It is corrected once Q29 is decided.

## 5. Decision log

v12's §10 stands, with these additions.

### 5.1 Decided

| # | Decision | Date |
|-|-|-|
| **Q27** | **Public tooling, private site inputs and evidence** (§2). Folia's repository holds the generator and the generic tools. A site's configuration, ledgers, toolchain records, reference manifest, site-specific checks and all evidence are kept by its owner and named by environment. The public build needs none of them | 2026-11-06 |
| **Q28** | **Repository replacement:** Folia's public repository is `ggirelli/folia-web`, with the sanitized history. The project keeps the name Folia. `ggirelli/folia` stays private and archived until its deletion, which comes only after the transition period and with explicit approval. Accepted on 2026-11-07; the transition period begins then (§1.4) | 2026-11-07 |

### 5.2 Open

| # | Question | Options and trade-offs | Recommendation | Decide by |
|-|-|-|-|-|
| **Q29** | Where Q26's evidence is committed, now that it is private | a private repository that holds Folia's public history and commits each evidence folder as the child of the Folia commit it was built from, so `just evidence-provenance` applies unchanged / a plain private archive, with the Folia commit recorded in each folder and the parentage check replaced by a record check | the private repository, so the existing check keeps working; it is never a remote of `folia-web` | before the next evidence run |

## 6. Unchanged

Everything else is v12's: the target architecture (v12 §3), the shared contracts (§4), the parity
strategy (§5), the roadmap A–D (§6), migration cleanup and cutover (§7), the acceptance criteria
F1–F16 (§8), risks and dependencies (§9; §9.3 now also lists the site's private inputs), the
decision log (§10, with §5 above) and the non-goals (§11).
