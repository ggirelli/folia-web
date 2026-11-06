"""The acceptance runner's records and verdict (plan v12, §5.4, work item A2.10; F12).

`scripts/accept.sh` runs the checks; this module never runs one. It has two commands:

- `lockfile BASELINE LABEL TOOLCHAIN_DIR`: prints the path of the exact `Gemfile.lock` the
  baseline was built with, after checking its SHA-256 against the one its `BASELINE.json`
  records: the baseline's own `Gemfile.lock` (`lockfile.sha256`, A2.7), or else, for a baseline
  that predates kept lockfiles, `TOOLCHAIN_DIR/local-LABEL.Gemfile.lock`
  (`site.gemfile_lock_sha256`). Exits 2, naming the problem, when there is none or it differs.
- `report EVIDENCE LABEL BASELINE GEMFILE_LOCK`: reads what the run left in EVIDENCE (the
  transcripts, the builds' records, the reports, the Folia tree's state before and after) and
  writes `accept.json` and `accept.md` there, neither overwritten. Exits 0 when every check ran
  and passed and no criterion failed, else 1.

Each criterion of plan v12, §8, is judged in parts, each `pass` (implemented, and its check
passed), `fail` (implemented, and its check failed) or `pending` (it cannot be judged yet: a
later step, or acceptance itself, must come first), with the check it comes from and why. A
criterion fails when a part fails, is pending when a part is pending, and passes only when every
part passes; `final_acceptance` is true only when every criterion passes. The run is
`evidence_eligible` only when the Folia tree was a clean commit before and after it and every
build recorded that commit, clean, with a known site identity: what Q26 requires of a build. A
run that is not eligible is diagnostic, whatever its verdict; one that is still has to pass
`just evidence-provenance` once it is committed as evidence (Q26).

Checks that plan v12 maps to none of §8's criteria (A2.8's links) are `other_checks`: mandatory,
gating the run, counted toward no criterion. `reference` records the determinism reference
`accept.sh` chose (`reference.txt`), or why it refused the check.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

# The checks `scripts/accept.sh` runs, in its order.
CHECKS = ["routes", "markdown", "pages", "determinism"]
PENDING = "pending"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lockfile(baseline: Path, label: str, toolchain: Path) -> int:
    record_path = baseline / "BASELINE.json"
    try:
        record = json.loads(record_path.read_text())
    except (OSError, ValueError) as error:
        print(f"{record_path}: {error}", file=sys.stderr)
        return 2
    kept = baseline / "Gemfile.lock"
    if kept.is_file():
        path, field = kept, "lockfile.sha256"
        recorded = (record.get("lockfile") or {}).get("sha256")
    else:
        path, field = toolchain / f"local-{label}.Gemfile.lock", "site.gemfile_lock_sha256"
        recorded = (record.get("site") or {}).get("gemfile_lock_sha256")
        if not path.is_file():
            print(
                f"{baseline} keeps no Gemfile.lock and {path} does not exist: the lockfile the "
                "baseline was built with is not known",
                file=sys.stderr,
            )
            return 2
    if not isinstance(recorded, str) or not re.fullmatch(r"[0-9a-f]{64}", recorded):
        print(f"{record_path} records no {field}", file=sys.stderr)
        return 2
    actual = sha256(path)
    if actual != recorded:
        print(
            f"{path} has the SHA-256 {actual}, {record_path} records {recorded} ({field})",
            file=sys.stderr,
        )
        return 2
    print(path)
    return 0


class Run:
    """What one run left in its evidence folder."""

    def __init__(self, evidence: Path):
        self.evidence = evidence
        self.transcripts: dict[str, list[str]] = {}
        self.exits: dict[str, int | None] = {}
        for check in CHECKS:
            path = evidence / "transcripts" / f"{check}.txt"
            lines = path.read_text().splitlines() if path.is_file() else []
            self.transcripts[check] = lines
            ends = [line for line in lines if re.fullmatch(r"# exit \d+", line)]
            self.exits[check] = int(ends[-1].split()[-1]) if ends else None
        self.claimed: dict[str, set[int]] = {check: set() for check in CHECKS}
        self.coverage = self.json("reports/pages/coverage.json")
        self.dom = self.json("reports/pages/dom-report.json")

    def json(self, relative: str):
        path = self.evidence / relative
        try:
            return json.loads(path.read_text())
        except (OSError, ValueError):
            return None

    def verdicts(self, check: str, subject: str, optional: bool = False) -> tuple[str, str]:
        """The `ok:` and `FAILS:` lines of `check` whose subject matches `subject`; with
        `optional`, a check that passed may give none (there was nothing of the kind to check)."""
        found = []
        for index, line in enumerate(self.transcripts[check]):
            match = re.match(rf"(ok|FAILS): ({subject})", line)
            if match:
                self.claimed[check].add(index)
                found.append((match.group(1), line))
        if not found and optional and self.exits[check] == 0:
            return "pass", f"`{check}` passed, and had nothing of this kind to check"
        if not found:
            return "fail", f"`{check}` reported no verdict on this ({self.ran(check)})"
        failed = [line for verdict, line in found if verdict == "FAILS"]
        if failed:
            return "fail", f"{len(failed)} of {len(found)} verdicts fail: " + "; ".join(failed)
        return "pass", f"{len(found)} verdicts, all ok: " + "; ".join(line for _, line in found)

    def ran(self, check: str) -> str:
        code = self.exits[check]
        return "it did not finish" if code is None else f"it exited with {code}"

    def exited(self, check: str, holds: str) -> tuple[str, str]:
        """`check`'s exit status and its closing line, which judge each of its verdicts."""
        for index, line in enumerate(self.transcripts[check]):
            if re.match(r"(ok|FAILS): ", line):
                self.claimed[check].add(index)
        closing = [line for line in self.transcripts[check] if line.startswith(holds)]
        if self.exits[check] == 0 and closing:
            return "pass", f"exit 0: {closing[-1]}"
        refused = [line for line in self.transcripts[check] if line.startswith("refused: ")]
        if refused:
            return "fail", f"`{check}` was not run: {refused[-1].removeprefix('refused: ')}"
        return "fail", f"`{check}`: {self.ran(check)}"

    def not_built(self) -> int | None:
        counts = (self.coverage or {}).get("counts") or {}
        value = counts.get("not_built_yet")
        return value if isinstance(value, int) else None


# A criterion's parts are (part, source check, plan v12 reference, judge); a judge takes the run
# and returns (status, why).
def pending(why: str):
    return lambda run: (PENDING, why)


def lines(check: str, subject: str, optional: bool = False):
    return lambda run: run.verdicts(check, subject, optional)


def not_built_part(name: str):
    """The outputs not built yet, which a criterion about every output covers only once built."""

    def judge(run: Run):
        count = run.not_built()
        if count is None:
            return "fail", "coverage.json gives no count of the outputs not built yet"
        if count:
            return PENDING, (
                f"{count} planned outputs are not built yet (coverage.json); {name} is checked on "
                "the outputs Folia builds, and covers the others as the B and C steps build them"
            )
        return "pass", "every planned output is built (coverage.json)"

    return ("the outputs not built yet", "pages: coverage.json", "§6 B, C; F1", judge)


def not_built_zero(run: Run):
    count = run.not_built()
    if count is None:
        return "fail", "coverage.json gives no count of the outputs not built yet"
    if count:
        return PENDING, f"not built yet = {count}; F1 requires 0 (built in the B and C steps)"
    return "pass", "not built yet = 0"


def markdown_gates(run: Run):
    rows = []
    for index, line in enumerate(run.transcripts["markdown"]):
        match = re.fullmatch(r"\| (G\d [^|]*?) \|.*\| ([^|]*?) \|", line)
        if match:
            run.claimed["markdown"].add(index)
            rows.append((match.group(1), match.group(2)))
    gates = {name.split()[0] for name, _ in rows}
    if gates != {f"G{n}" for n in range(1, 7)}:
        return (
            "fail",
            f"the report gives gates {sorted(gates)}, not G1 to G6 ({run.ran('markdown')})",
        )
    failed = [f"{n}: {r}" for n, r in rows if not re.fullmatch(r"(.*; )?pass", r)]
    if failed or run.exits["markdown"] != 0:
        return "fail", f"{run.ran('markdown')}; " + "; ".join(failed or ["no gate fails"])
    return "pass", "exit 0; " + "; ".join(f"{n}: {r}" for n, r in rows)


def rouge_floor(run: Run):
    measured = [
        line for line in run.transcripts["markdown"] if line.startswith("| **lexed by Rouge**")
    ]
    agreement = re.findall(r"\*\*([\d.]+%)\*\*", measured[-1]) if measured else []
    seen = f"reported, not enforced: {agreement[-1]} site-wide" if agreement else "not reported"
    return PENDING, f"{seen}; the floor of 90% (Q12) and its negative mutation are A2's to build"


def unbuilt_scopes(run: Run) -> dict[str, int] | None:
    """How many outputs not built yet each §5.1 assertion scope holds: `pages` (HTML outputs that
    are not redirect stubs), `stubs` (redirect stubs) and `root` (`/`, `index.html`); None when
    the reports cannot tell."""
    rows = (run.coverage or {}).get("rows")
    planned = re.findall(
        r"Redirect stubs \(URL, output\): Jekyll (\d+)", "\n".join(run.transcripts["routes"])
    )
    built = re.findall(
        r"Folia built \d+ pages, (\d+) redirect stubs", "\n".join(run.transcripts["pages"])
    )
    if not isinstance(rows, list) or not planned or not built:
        return None
    unbuilt = [r.get("output", "") for r in rows if r.get("class") == "not-built-yet"]
    stubs = int(planned[-1]) - int(built[-1])
    html = sum(1 for output in unbuilt if output.endswith(".html"))
    return {"pages": html - stubs, "stubs": stubs, "root": int("index.html" in unbuilt)}


# The hard assertions of §5.1, each with the outputs its "Applies to" names, as plan v12 words it,
# and the scope of `unbuilt_scopes` holding those of them not built yet.
ASSERTIONS = [
    ("robots", "every HTML page Jekyll gives one", "pages"),
    ("canonical", "every HTML page", "pages"),
    ("title", "every HTML page", "pages"),
    ("description", "every HTML page with one", "pages"),
    ("lang", "every HTML page", "pages"),
    ("rel-me", "`/`", "root"),
    ("h-card", "`/`", "root"),
    ("h-entry", "entry pages (as built: a page Jekyll gives an `h-entry`)", "pages"),
    ("stub", "redirect stubs", "stubs"),
]
SCOPES = {
    "pages": (
        "{n} HTML pages other than redirect stubs are not built yet (fragments, which §5.1 exempts "
        "from the document-level assertions, among them); those it applies to are judged when "
        "built"
    ),
    "stubs": "{n} redirect stubs are not built yet",
    "root": "`/` (`index.html`) is not built yet (B2)",
}


def assertion(name: str, applies: str, scope: str):
    """The §5.1 assertion `name`: its outcomes on the outputs built, and the outputs of its scope
    not built yet."""

    def judge(run: Run):
        found = (run.dom or {}).get("assertions")
        if not isinstance(found, list):
            return "fail", "dom-report.json lists no assertions"
        outcomes = [a for a in found if a.get("assertion") == name]
        failed = [a.get("file") for a in outcomes if a.get("passed") is not True]
        if failed:
            return "fail", f"{len(failed)} of {len(outcomes)} fail: {', '.join(failed)}"
        files = ", ".join(a.get("file") for a in outcomes)
        held = (
            f"holds on the {len(outcomes)} built it applies to ({files})"
            if outcomes
            else "has not run"
        )
        scopes = unbuilt_scopes(run)
        if scopes is None:
            return "fail", f"{held}; the reports do not tell which outputs it applies to are built"
        if scopes[scope]:
            return PENDING, f"{held}; {SCOPES[scope].format(n=scopes[scope])}"
        if not outcomes and scope in ("root", "stubs"):
            return "fail", f"it applies to {applies} (§5.1), which is built, and it did not run"
        return "pass", f"{held}; every output it applies to is built"

    return (f"`{name}`", "pages: dom-report.json", f"§5.1: {applies}", judge)


# Where plan v12's wording leaves a part's reading open: kept as stated, for the owner to decide.
AMBIGUITIES = {
    ("F6", "`h-entry`"): (
        "§5.1 applies it to 'entry pages'; the check applies it to every page Jekyll gives an "
        "`h-entry`, pages such as /about included. This part follows the check; whether §5.1 "
        "means the narrower set is the owner's to say."
    ),
    ("F10", "the warnings of the outputs not built yet"): (
        "F10 asks that every warning match the file and every entry a warning, without naming the "
        "build; §8 runs the criteria on the final history, when every output is built. Read so, "
        "the warnings of the notes not built yet are pending; read as the warnings of the "
        "current build only, this part would not exist. Kept pending for the owner to confirm."
    ),
}


def unbuilt_warnings(run: Run):
    count = run.not_built()
    if count is None:
        return "fail", "coverage.json gives no count of the outputs not built yet"
    if not count:
        return "pass", "every planned output is built, so every warning the site gives was judged"
    return PENDING, (
        "the check judges the warnings this build gives; Folia gives a note's broken wiki-link "
        "warning only when it builds the note (crates/garden/src/note.rs: a built note's broken "
        f"wiki-link is a warning), so the notes among the {count} outputs not built yet have given "
        "none, and their warnings are judged as the B and C steps build them"
    )


def ledger_final(run: Run):
    ledger = (run.dom or {}).get("ledger")
    if not isinstance(ledger, dict) or not isinstance(ledger.get("final_acceptance"), list):
        return "fail", "dom-report.json has no ledger.final_acceptance"
    refused = ledger["final_acceptance"]
    if refused:
        return "fail", f"final acceptance would refuse: {'; '.join(refused)}"
    return "pass", (
        "nothing final acceptance would refuse (no proposed entry, no open follow-up, none "
        "stale, every entry bound as reviewed: dom-report.json ledger.final_acceptance)"
    )


def record_written(run: Run):
    needed = [
        "inputs/folia.toml",
        "lockfiles/Cargo.lock",
        "lockfiles/Gemfile.lock",
        "baseline/BASELINE.json",
        "builds/pages/build.json",
    ]
    missing = [p for p in needed if not (run.evidence / p).is_file()]
    if missing:
        return "fail", f"missing from the evidence folder: {', '.join(missing)}"
    return (
        "pass",
        "this folder: commits, configuration, templates, build time, toolchain, lockfiles",
    )


CRITERIA = [
    (
        "F1",
        "Every output accounted for",
        [
            (
                "every row classified, none unclassified, no problem",
                "pages",
                "§5.4 A2.1",
                lines("pages", "coverage of"),
            ),
            ("not built yet = 0", "pages: coverage.json", "§8 F1", not_built_zero),
            (
                "no `[build]` table in the shipped configuration",
                "none yet",
                "§4.1; D2 (Q8)",
                pending("the shipped configuration is the site's `_folia/folia.toml`, made in D2"),
            ),
        ],
    ),
    (
        "F2",
        "No unplanned outputs",
        [
            (
                "builder check and route check of every output written",
                "pages",
                "§5.4 A2.2",
                lines("pages", "routes of every output written"),
            )
        ],
    ),
    (
        "F3",
        "Routes",
        [
            (
                "authored, classification, generated, redirect stubs",
                "routes",
                "spike 2",
                lambda run: run.exited("routes", "Route parity holds."),
            ),
            (
                "the *claimed* class and `missing_templates`",
                "none yet",
                "§3.4; A2",
                pending("route parity's claimed class is A2's, not built yet"),
            ),
            (
                "every *removed* row planned as removed",
                "none yet",
                "§8 F3",
                pending("route parity does not check removals yet"),
            ),
        ],
    ),
    (
        "F4",
        "Byte-exact outputs",
        [
            (
                "redirect stubs and static files",
                "pages",
                "§5.3",
                lines("pages", "redirect stubs and static files byte-identical"),
            ),
            ("the note graph", "pages", "§5.3", lines("pages", r"assets/json/notes_graph\.json")),
            (
                "CSS and source maps",
                "none yet",
                "§5.3; C5",
                pending("C5's comparators, not built yet"),
            ),
            not_built_part("F4"),
        ],
    ),
    (
        "F5",
        "HTML",
        [
            (
                "the §5.1 whole-document comparison, every difference an accepted entry's",
                "pages",
                "§5.1, §5.2",
                lines("pages", "whole-document DOM comparison"),
            ),
            (
                "page checks: fingerprint, main content, table of contents",
                "pages",
                "§5.1",
                lines(
                    "pages",
                    r"\S+\.html: (fingerprint|div\.main-content|table of contents)",
                ),
            ),
            (
                "entries reviewed manually",
                "the ledger",
                "§5.2",
                pending(
                    "manual, and not judged here: `just accept` records no review, and its "
                    "automatic parts passing does not mean the entries were reviewed"
                ),
            ),
            not_built_part("F5"),
        ],
    ),
    (
        "F6",
        "Document assertions",
        [
            *(assertion(name, applies, scope) for name, applies, scope in ASSERTIONS),
        ],
    ),
    (
        "F7",
        "Generated outputs",
        [
            (
                "feeds, sitemap, redirects.json, search, CSS and source maps",
                "none yet",
                "§5.3; C2–C5",
                pending("the C2 to C5 comparators, not built yet"),
            )
        ],
    ),
    (
        "F8",
        "Markdown and content",
        [
            ("G1 to G6, accepted entries only", "markdown", "spike 1", markdown_gates),
            ("Rouge class agreement ≥ 90%, enforced", "markdown", "Q12; A2", rouge_floor),
            (
                "highlighted blocks of the pages built",
                "pages",
                "spike 1",
                lines("pages", r"\S+\.html: \d+ highlighted blocks", optional=True),
            ),
            (
                "dates equal the dump",
                "pages",
                "§8 F8",
                lines("pages", r"\S+: dates equal the baseline's dump"),
            ),
            not_built_part("F8's dates"),
        ],
    ),
    (
        "F9",
        "Front matter and unsupported features",
        [
            (
                "0 unclassified keys; every zero-use check",
                "none yet",
                "§5.5; A3",
                pending("A3, not started"),
            )
        ],
    ),
    (
        "F10",
        "Warnings",
        [
            (
                "every warning of this build expected, every entry matched",
                "pages",
                "§5.4 A2.9",
                lines("pages", "expected warnings:"),
            ),
            (
                "one warning per broken link",
                "pages",
                "§5.4 A2.9",
                lines("pages", "one warning per broken link"),
            ),
            (
                "the warnings of the outputs not built yet",
                "pages: coverage.json",
                "§8 F10; crates/garden/src/note.rs",
                unbuilt_warnings,
            ),
        ],
    ),
    (
        "F11",
        "Determinism",
        [
            (
                "twice in one tree, byte for byte, and against the baseline's reference",
                "determinism",
                "§5.4 A2.5",
                lambda run: run.exited("determinism", "Determinism holds"),
            ),
            (
                "from two fresh clones of the final commits",
                "none yet",
                "§5.4 A2.5; D3",
                pending("reproducibility across checkouts: runs at acceptance (D3)"),
            ),
        ],
    ),
    (
        "F12",
        "Reproducibility",
        [
            ("one evidence folder with every input", "accept", "§5.4 A2.10", record_written),
            (
                "runs F1–F11, F13 and F15",
                "accept",
                "§8 F12; D3",
                pending("completed in D3: it runs the checks built so far"),
            ),
            (
                "the inputs are commits pushed under Q19",
                "none",
                "§8 F12; Q19",
                pending("manual: checked by hand"),
            ),
        ],
    ),
    (
        "F13",
        "Ledger",
        [
            (
                "what final acceptance refuses of the DOM ledger",
                "pages: dom-report.json",
                "§5.2",
                ledger_final,
            ),
            not_built_part("F13"),
        ],
    ),
    (
        "F14",
        "Visual review",
        [
            (
                "the sample at two viewports, and the author's sign-off",
                "none",
                "§8 F14; A2 (viewports), D3",
                pending("manual, at D3; the two viewports are to be fixed in A2"),
            )
        ],
    ),
    (
        "F15",
        "Production comparison",
        [
            (
                "F1–F11 against production's own Jekyll artifact",
                "none",
                "§8 F15; D3; Q19",
                pending("D3; the production artifact needs its own authorization (Q19)"),
            )
        ],
    ),
    (
        "F16",
        "Operations and approval",
        [
            (
                "deploy, rollback and documentation reviewed; the author's approval",
                "none",
                "§8 F16; D3",
                pending("manual, at D3; not run by `just accept` (F12)"),
            )
        ],
    ),
]


# Checks built so far that decide the run but are none of §8's criteria: each (check, source,
# plan v12 reference, judge), as a criterion's parts.
OTHERS = [
    (
        "A2.8 route-level links: every internal href and src resolves",
        "pages",
        "§5.4 A2.8 (begins D1)",
        lines("pages", "links:"),
    )
]
OTHER_ROLE = (
    "mandatory: a failure fails the run; mapped to none of §8's criteria, as plan v12 maps it "
    "to none"
)


def reference(evidence: Path) -> dict:
    """The determinism reference `accept.sh` chose (`reference.txt`), or why it chose none."""
    path = evidence / "reference.txt"
    fields = {}
    for line in path.read_text().splitlines() if path.is_file() else []:
        key, _, value = line.partition(": ")
        fields[key] = value
    if "path" in fields:
        copy = evidence / "inputs" / "reference-sha256.txt"
        fields["copy"] = "inputs/reference-sha256.txt"
        fields["sha256"] = sha256(copy) if copy.is_file() else None
    return fields


def status_of(parts: list[dict]) -> str:
    statuses = {part["status"] for part in parts}
    if "fail" in statuses:
        return "fail"
    return PENDING if PENDING in statuses else "pass"


def folia_state(path: Path) -> dict:
    """A `folia-*.txt` record: the commit on its first line, then `git status` lines."""
    lines = path.read_text().splitlines() if path.is_file() else []
    commit = lines[0] if lines else None
    changes = lines[1:]
    return {"commit": commit, "state": "changed" if changes else "clean", "changes": changes}


def eligibility(evidence: Path, before: dict, after: dict) -> list[str]:
    """Why the run is not Q26 evidence; empty when it can be."""
    reasons = []
    if before["changes"]:
        reasons.append(f"the Folia tree had changes before the run: {', '.join(before['changes'])}")
    if after["changes"]:
        reasons.append(f"the Folia tree had changes after the run: {', '.join(after['changes'])}")
    if before["commit"] != after["commit"]:
        reasons.append(
            f"Folia's commit changed during the run: {before['commit']}, {after['commit']}"
        )
    records = sorted((evidence / "builds").glob("*/build.json"))
    if not records:
        reasons.append("no build record was kept")
    for record in records:
        name = record.parent.name
        try:
            data = json.loads(record.read_text())
        except ValueError:
            reasons.append(f"builds/{name}: build.json is not JSON")
            continue
        folia, site = data.get("folia") or {}, data.get("site") or {}
        if folia.get("commit") != before["commit"]:
            reasons.append(f"builds/{name}: built from Folia {folia.get('commit')}")
        if (folia.get("changes") or {}).get("state") != "clean":
            reasons.append(f"builds/{name}: Folia not clean: {folia.get('changes')}")
        if (site.get("changes") or {}).get("state") not in ("clean", "changed"):
            reasons.append(f"builds/{name}: the site's identity is unknown: {site.get('changes')}")
    return reasons


def report(evidence: Path, label: str, baseline: Path, gemfile_lock: Path) -> int:
    for name in ("accept.json", "accept.md"):
        if (evidence / name).exists():
            print(f"{evidence / name} exists; it is never overwritten", file=sys.stderr)
            return 2
    run = Run(evidence)
    criteria = []
    for ident, title, parts in CRITERIA:
        judged = []
        for name, source, plan, judge in parts:
            status, why = judge(run)
            judged.append(
                {"part": name, "status": status, "source": source, "plan": plan, "why": why}
            )
            if (ident, name) in AMBIGUITIES:
                judged[-1]["ambiguity"] = AMBIGUITIES[(ident, name)]
        criteria.append({"id": ident, "title": title, "status": status_of(judged), "parts": judged})
    others = []
    for name, source, plan, judge in OTHERS:
        status, why = judge(run)
        others.append(
            {
                "check": name,
                "status": status,
                "required": True,
                "gates_run": True,
                "criterion": None,
                "role": OTHER_ROLE,
                "source": source,
                "plan": plan,
                "why": why,
            }
        )
    checks = [
        {
            "name": check,
            "transcript": f"transcripts/{check}.txt",
            "command": next(
                (line[2:] for line in run.transcripts[check] if line.startswith("$ ")), None
            ),
            "exit": run.exits[check],
            "passed": run.exits[check] == 0,
            "unjudged": [
                line
                for index, line in enumerate(run.transcripts[check])
                if re.match(r"(ok|FAILS): ", line) and index not in run.claimed[check]
            ],
        }
        for check in CHECKS
    ]
    before = folia_state(evidence / "folia-before.txt")
    after = folia_state(evidence / "folia-after.txt")
    reasons = eligibility(evidence, before, after)
    pages = run.json("builds/pages/build.json") or {}
    record = json.loads((baseline / "BASELINE.json").read_text())
    toolchain = {}
    for line in (evidence / "toolchain.txt").read_text().splitlines():
        key, _, value = line.partition(": ")
        toolchain[key] = value
    lockfiles = [
        {"file": "lockfiles/Cargo.lock", "source": "Cargo.lock"},
        {"file": "lockfiles/Gemfile.lock", "source": str(gemfile_lock)},
    ]
    for lock in lockfiles:
        lock["sha256"] = sha256(evidence / lock["file"])
    lockfiles[1]["recorded_sha256"] = (record.get("lockfile") or {}).get("sha256") or (
        record.get("site") or {}
    ).get("gemfile_lock_sha256")
    passed = (
        all(c["passed"] for c in checks)
        and all(c["status"] != "fail" for c in criteria)
        and all(o["status"] != "fail" for o in others)
    )
    final = all(c["status"] == "pass" for c in criteria)
    data = {
        "format": 1,
        "label": label,
        "passed": passed,
        "final_acceptance": final,
        "evidence_eligible": not reasons,
        "not_eligible_because": reasons,
        "folia": {"before": before, "after": after},
        "site": pages.get("site"),
        "baseline": {
            "label": label,
            "path": str(baseline),
            "record": "baseline/BASELINE.json",
            "commit": (record.get("site") or {}).get("commit"),
            "created_at": record.get("created_at"),
            "tools": record.get("tools"),
            "build": record.get("build"),
        },
        "config": pages.get("config"),
        "templates": pages.get("templates"),
        "build_time": pages.get("at"),
        "toolchain": toolchain,
        "reference": reference(evidence),
        "lockfiles": lockfiles,
        "checks": checks,
        "criteria": criteria,
        "other_checks": others,
    }
    (evidence / "accept.json").write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    (evidence / "accept.md").write_text(markdown(data))
    for criterion in criteria:
        print(f"{criterion['id']} {criterion['title']}: {criterion['status']}")
    for check in checks:
        for line in check["unjudged"]:
            print(f"note: {check['name']}: a verdict no criterion judges: {line}")
    verdict = "the checks built so far pass" if passed else "FAILS"
    print(
        f"{verdict}; final acceptance: {'met' if final else 'not met'}; "
        f"evidence eligible: {'yes' if not reasons else 'no'} (see {evidence / 'accept.md'})."
    )
    return 0 if passed else 1


def markdown(data: dict) -> str:
    yes = lambda value: "yes" if value else "**no**"
    out = [
        f"# Acceptance run on `{data['label']}`",
        "",
        (
            "Written by `just accept` (plan v12, §5.4, A2.10; F12). `pass`: implemented and passed;"
            " `fail`: implemented and failed; `pending`: not yet judgeable."
        ),
        "",
        "| | |",
        "|-|-|",
        f"| The checks built so far pass | {yes(data['passed'])} |",
        f"| Final acceptance (every criterion passes) | {yes(data['final_acceptance'])} |",
        f"| Evidence eligible (Q26) | {yes(data['evidence_eligible'])} |",
        (
            "| Other checks (mandatory, outside §8) pass | "
            f"{yes(all(o['status'] == 'pass' for o in data['other_checks']))} |"
        ),
    ]
    if data["not_eligible_because"]:
        out += ["", "**Not evidence**: a diagnostic run, because:", ""]
        out += [f"- {reason}" for reason in data["not_eligible_because"]]
    folia = data["folia"]["before"]
    site = data["site"] or {}
    templates = data["templates"] or {}
    out += [
        "",
        "## Inputs",
        "",
        "| Input | Value |",
        "|-|-|",
        f"| Folia | `{folia['commit']}`, {folia['state']} |",
        f"| Site | `{site.get('commit')}`, {json.dumps(site.get('changes'))} |",
        (
            f"| Baseline | `{data['label']}`, built {data['baseline']['created_at']} "
            "(`baseline/BASELINE.json`) |"
        ),
        (
            f"| Configuration | `{(data['config'] or {}).get('path')}`, "
            f"`{(data['config'] or {}).get('sha256')}` |"
        ),
        f"| Templates | `{templates.get('tree_sha256')}` ({templates.get('files')} files) |",
        f"| Build time | `{data['build_time']}` |",
        (
            f"| Determinism reference | `{data['reference'].get('path')}`, "
            f"`{data['reference'].get('sha256')}` ({data['reference'].get('chosen')}) |"
            if "path" in data["reference"]
            else f"| Determinism reference | **none**: {data['reference'].get('refused')} |"
        ),
    ]
    out += [f"| {key} | `{value}` |" for key, value in data["toolchain"].items()]
    out += [f"| `{lock['file']}` | `{lock['sha256']}` |" for lock in data["lockfiles"]]
    out += ["", "## Checks", "", "| Check | Exit | Transcript |", "|-|-|-|"]
    out += [
        f"| `{c['command']}` | {c['exit']} | [`{c['transcript']}`]({c['transcript']}) |"
        for c in data["checks"]
    ]
    out += ["", "## Criteria (plan v12, §8)", "", "| # | Part | Status | Source | Plan | Why |"]
    out.append("|-|-|-|-|-|-|")
    for criterion in data["criteria"]:
        out.append(
            f"| **{criterion['id']}** | **{criterion['title']}** | **{criterion['status']}** "
            "| | | |"
        )
        for part in criterion["parts"]:
            why = part["why"].replace("|", "\\|")
            out.append(
                f"| | {part['part']} | {part['status']} | {part['source']} | {part['plan']} "
                f"| {why} |"
            )
    open_parts = [
        (criterion["id"], part["part"], part["ambiguity"])
        for criterion in data["criteria"]
        for part in criterion["parts"]
        if "ambiguity" in part
    ]
    if open_parts:
        out += ["", "## Readings open for the owner", ""]
        out += [f"- **{ident}**, {name}: {text}" for ident, name, text in open_parts]
    out += [
        "",
        "## Other checks",
        "",
        (
            "Mandatory checks that plan v12 maps to none of §8's criteria: each gates the run (a"
            " failure fails it), and none is counted toward any criterion."
        ),
        "",
        "| Check | Status | Mandatory, gates the run | Source | Plan | Why |",
        "|-|-|-|-|-|-|",
    ]
    for other in data["other_checks"]:
        why = other["why"].replace("|", "\\|")
        gates = yes(other["required"] and other["gates_run"])
        out.append(
            f"| {other['check']} | {other['status']} | {gates} | {other['source']} "
            f"| {other['plan']} | {why} |"
        )
    unjudged = [(c["name"], line) for c in data["checks"] for line in c["unjudged"]]
    if unjudged:
        out += [
            "",
            "## Verdicts nothing above judges",
            "",
            "They decide the run through their check's exit status.",
            "",
        ]
        out += [f"- `{name}`: {line}" for name, line in unjudged]
    return "\n".join(out) + "\n"


def main(argv: list[str]) -> int:
    if len(argv) == 4 and argv[0] == "lockfile":
        return lockfile(Path(argv[1]), argv[2], Path(argv[3]))
    if len(argv) == 5 and argv[0] == "report":
        return report(Path(argv[1]), argv[2], Path(argv[3]), Path(argv[4]))
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
