# Two authors, one trunk — a working agreement (proposal)

Status: **proposal for discussion, unratified.** Nothing in this file is policy
until both names are on it. Written 2026-09-27 from the public record of this
repository — commits, pull requests, issues, rulesets and the documents in
`docs/`. Every number below is reproducible with the commands in Appendix A.
Where a claim is an inference rather than a measurement, it says so.

It is not a performance review, a truce, or a verdict about who is right. It is
an interface specification for two working styles that do not, on inspection,
actually conflict — they conflict only because this repository has one trunk and
no seam between them.

---

## 1. The two working styles, taken at their strongest

Both are legitimate engineering ethics. Each is the correct answer to a
different question, and each is actively bad at the other's question.

| | **Frontier** (Jonathan / @hyperpolymath) | **Trunk** (Joshua / @JoshuaJewell) |
|---|---|---|
| Unit of work | a *claim about the system* — a gate, a proof, a boundary, a contract | a *working package* — something a person can run, install, play |
| Definition of done | the claim is checkable and can fail loudly when false | the thing works on a machine that is not yours |
| Natural cadence | many parallel threads; agents; breadth; spikes | one component at a time; complete; integrated |
| Tolerance for red | a red branch is information | a red `main` is a broken product |
| Reaches for | ADR, fixture, planted failure, cross-repo contract | reproduce, patch, ship, next |
| Protects the project from | claims that were never true and never checked | software that is theoretically complete and does not run |
| Weakness when alone | a build that compiles and proves things nobody can play | a fix nobody can build on, or find |
| In their own words | *"The method is deliberately conservative: small, formally grounded cores, with larger systems built around them."* (his GitHub profile README) · *"A check that cannot fail is not a check."* (`AGENTS.md`) | *"Quick Start fails on fresh clone"* (`burble` #27) · *"playground/ is scaffolded but has no source — `just dev`/`just test` fail"* (`betlang` #6) · *"Fixed install and run on windows"* (commit `c9787ae6`) |

Read together, the two columns describe one good project: somebody who insists
the claim is real, and somebody who insists the artefact runs. Read apart, each
column reads as the other's failure mode. That is the whole disagreement.

## 2. What the repository measures

| Measurement | Value |
|---|---|
| Commits on `main` | 245 — **214** Jonathan, **7** Joshua (4 as `JoshuaJewell`, 3 as `Joshua Jewell`), 24 bots |
| Pull requests, all time | 120 — **92** Jonathan, **1** Joshua (`#16`), 27 bots |
| `#16` by size | +5,499 / −182 across 59 files — the **5th largest** PR in the repo's history |
| Merges | 115 merged; **110 merged by Jonathan**, 4 by Mergify, **1 by Joshua** — of a *bot's* PR (`#114`) |
| PRs merged by their own author | **90 of 120** |
| PRs with zero reviews | **86 of 120** |
| Median time, PR opened → merged | **14 minutes** (49 of 115 merged in under 10; 86 in under an hour) |
| Issues | 29, **all opened by Jonathan**, **0 open**; 0 open PRs; 1 discussion, the welcome post |
| Direct pushes to `main` without a PR | both authors (4 commits by Joshua: `df057f0f`, `c540895a`, `8a881861`, `c9787ae6`) |
| Governance | `.github/MAINTAINERS`: **one** active maintainer. `.github/CODEOWNERS`: `* @hyperpolymath`. Contribution profile: *"Core (single maintainer today; no Trusted-contributor or Community-sandbox tier established yet)"* |
| Assurance profile, named residual risk | *"single maintainer means bus-factor risk on review capacity"* |
| Security and conduct contact, in all four files that name one | `developer@joshuajewell.dev` — while `DR-0002` asks whether that or `jonathan.jewell@gmail.com` is the monitored inbox |
| Project origin | first commit `08302627` — *"Initial commit"*, 2026-06-25, **Joshua**. Canonical repo created 2026-07-10, five days after Jonathan's first commit into that same history (2026-07-04) |
| What is built | 6 crates, ~40,300 lines of Rust, deterministic event-sourced core, TUI, Bevy front end, delay-lockstep netplay, C ABI + Idris2 model, 10 workflows, a Creusot proof kernel under `verif/` |

None of that is a complaint. It is the shape of a repository where one person
can act and the other can only react, and it was reached by two people both
doing defensible work in good faith.

It is also the evidence for *"we could do something amazing"* being a
measurement rather than a hope. Six crates, a deterministic core with byte-
parity gates, working two-seat netplay, a player-facing renderer, a proof kernel
and a one-script install is most of the hard part of a game already built. The
risk that remains in this project is not technical.

### The case study: PR #16

Opened 2026-07-16 20:45Z — 5,499 lines fusing the grounded network into the
Ghost Lobby scenario. Then, on 2026-07-17:

```
09:38–09:39Z   three fix commits pushed onto the author's branch by the reviewer
09:42Z         "Ran a multi-agent adversarial review over the full diff (5 dimensions…
                12 findings confirmed, all fixed on this branch"
09:43Z         @mergifyio queue
09:45Z         merged — by the reviewer
```

No review was recorded. The author never commented on the PR, before or after.

**Both readings of that are correct.** From the frontier: the review found
twelve real defects, one of which (`hacker_pivot` mutating event-sourced state
outside the command stream) would have broken replay determinism — the load-
bearing invariant of the entire project. That was careful work, done fast, by
someone with the whole architecture in his head. From the trunk: five and a half
thousand lines were rewritten, finished and merged inside seven minutes,
thirteen hours after the PR was opened, by someone else, with no point at which
the author was asked anything.

Nothing in the repository made either of those readings avoidable. That is what
this document is for.

## 3. Why it keeps happening — six mechanisms

**M1. One trunk, two currencies.** The laboratory and the shipped surface are
the same branch. Every experiment is therefore priced in the other person's
currency, and every release-quality increment is a delay to the other's
experiment.

**M2. The only available review action is the one that carries merge
authority.** With one maintainer, `* @hyperpolymath` in `CODEOWNERS` and no
required review, the fastest way to complete a review is to fix the branch and
merge it. A review *comment* has no mechanism behind it, so "review" and
"rewrite" collapse into one action. 90 of 120 PRs were merged by their author;
86 had no reviewer at all.

**M3. Disagreements are filed as evidence questions.** `docs/decisions-pending/`
is excellent machinery — question, why it is not decidable here, options with
costs, guard, ruling. But it is tuned for *unknowns*, not for *disagreements
between the two people who own the repository*. `DR-0002` is the proof: it is a
genuine disagreement about who owns a relationship with the world (the security
inbox), filed as a question about whether a mailbox exists — and the guard's own
comment records what happened: *"A live contradiction… Neither mentioned the
other."* There is no register in which the two of you can be seen to disagree on
purpose, so disagreements do not become queue items. They become changes already
on `main`.

**M4. The canonical story is maintained by one of the two authors, and it has
drifted.** `AGENTS.md` states that `README.md` documents four incarnations —
the canonical `README.md` has no lineage section at all (`grep -i lineage
README.md` → nothing). The table in `AGENTS.md` records attempt 2 as
*"AffineScript / PixiJS"*; the earliest record of that table — `README.md` in
the repository where the lineage was first written, `JoshuaJewell/IDAprUSTIK` —
gives attempt 2 as **ReScript / PixiJS** and attempt 3 as AffineScript / PixiJS.
Two rows were flattened into one. Meanwhile `AGENTS.md` calls attempts 1–3
"dead" while `main` carries a commit that ports vector sprites from one of them
(*"Add interactive net view with vector sprites ported from
JoshuaJewell/IDApixiTIK"*, 2026-07-24).

A small error in a file whose audience is agents. Also the reason a conversation
about the engine starts one translation step behind where it should: the version
of the project's past in the tree is not the version of the project's past that
its originator holds, and only one of them is in the tree.

**M5. The three intersections where the two styles must actually meet are
unnamed.** They are not the whole repository; they are the C-ABI/Idris2
boundary (`crates/idaptik-ffi`, `contracts/`), the path that decides whether
anyone can run the thing at all (`install.sh`, `launcher.sh`, `just doctor`),
and the player-facing surface (`crates/idaptik-bevy`). These are exactly where
the two working styles are each half of the answer — and the repository assigns
all three, by `CODEOWNERS`, to one person.

**M6. Asymmetric consequence, so no symmetric venue.** One author can merge
anything; the other can block nothing. A disagreement therefore cannot be
expressed where it arises. It surfaces later, as a reaction to something already
on `main` — which reads, from the other side, as obstruction after the fact.

*Inference, marked as such:* the mechanisms above are read from artefacts, not
from anyone's account of them. M5's ordering of what matters and M6's claim that
disagreement surfaces late rather than early are interpretations of the record
rather than features of it. If either is wrong, that is an argument worth having
— in writing, on a numbered row.

## 4. The agreement: trunk, frontier, seam

**Two lanes, named.**

- **Trunk** — `main`. The package. Contract: it runs, and it is green, and
  "green" means the checks ran. Slow, reviewed, boring on purpose.
- **Frontier** — everything else: branches, forks, agent workspaces, `spike/`,
  `verif/`, `agent/`. Unlimited, fast, multi-agent, private-by-default, allowed
  to be red, requiring nobody's approval.

That costs nothing to adopt, because both lanes already exist; what is missing
is the rule at the seam.

**The seam rules.**

- **R1.** Frontier work reaches trunk only as a pull request. Branch names carry
  the lane.
- **R2.** One change, one author, one reviewer. The reviewer's job is to say
  yes, no, or here is what I would need — not to finish it. A reviewer who wants
  to fix the branch says so in a comment and waits for a yes; the hand-off is
  recorded on the PR. The author's name stays on it.
- **R3.** Nothing enters trunk unreviewed, including the maintainer's own work.
  If there is no available reviewer, it waits in the frontier or it does not
  land. *This rule binds both of you or it binds neither*: 90 of 120 PRs were
  self-merged, and the four direct pushes to `main` in §2 are one author's
  version of the same shortcut.
- **R4.** A check that was skipped, neutral, or never started is not a pass, and
  a gate that fires is not overridden to land a change. `DR-0004` measured the
  current position precisely: `#148` merged with three checks red, seven still
  running and one that never started, because the condition that was supposed to
  prevent it existed in a comment rather than in the config. Option A of that DR
  is the real fix and needs estate rights; R4 is what you do with your hands in
  the meantime.
- **R5.** A disagreement becomes a numbered row, not a re-litigated paragraph.
  Use `docs/decisions-pending/` for disagreements too, with two added fields:
  `owner:` (who must rule) and `rule-by:` (a date). Both positions in their
  authors' own words, options with costs, guard. `docs/dev-notes/roadmap-2026-07.md`
  already does this and it *works* — §2 of that file records "Joshua's concern:
  …" next to "Jonathan's argument: …" and both survive to the page. That format
  is the precedent; scale it. A disagreement with a number and a date is a queue
  item. Without one it is an argument.
- **R6.** The frontier reports; the trunk does not guess. Once per milestone
  boundary: one page in `docs/dev-notes/` — what is being tried, what is green
  and what is red, what it wants from `main`, and what it will cost `main` to
  carry. The shape already exists (`roadmap-2026-07.md`, `ESTATE-STATUS.md`).
- **R7.** Both names, with scopes. `MAINTAINERS` and `CODEOWNERS` reflect the
  two lanes rather than `*`. Proposed diff in Appendix B. This is the cheapest
  change available and the one with the largest effect: it converts "who is
  allowed to have an opinion" into a review request.

**What each side gives up** — an agreement that costs one side nothing is not an
agreement.

| | Gives up | Keeps, and gets |
|---|---|---|
| **Frontier** | rewriting another author's branch; self-merging; landing over a red check; treating "does it run?" as a less serious question than "is it true?" | breadth, speed, agents, red branches, the estate, the proofs, and the exclusive right to say *this claim is not checkable yet* |
| **Trunk** | landing anything unreviewed, including quick fixes; treating the architecture documents as the product; letting the only answer to "why is this here?" be a document | `main` always runnable; first refusal on anything touching a player's first five minutes; a queue of decisions to rule instead of a stream of changes to chase |

**Both** give up the private version of the story. The lineage in `AGENTS.md`,
the contact in `DR-0002`, the assumption in the assurance profile that there is
one maintainer — these are one-line answers that only the two of you can give,
and they currently cost more than every process rule above.

## 5. First moves (a week, not a quarter)

1. **Rule DR-0002.** One line from each of you: which inbox is read, and who is
   the fallback. It is the smallest possible demonstration that the queue
   mechanism works, and its guard is already written and firing.
2. **Fix the lineage.** Correct the attempt-2 row in `AGENTS.md`, either put the
   table back into `README.md` or drop the claim that it is there, and decide
   whether the IDApixiTIK sprite port is an asset pipeline (then "dead" is the
   wrong word for it) or an accident (then the port needs a home).
3. **Add the second name** to `MAINTAINERS`, `CODEOWNERS` and the contribution
   profile, and update the assurance profile's residual-risk line in the same
   commit — the house rule is that evidence labels must not drift, and *"single
   maintainer"* stops being true the moment this lands.
4. **Rule `DR-0004`** with the estate (option A, B or C) so R4 has an owner
   outside this repository.
5. **Run one PR each way under R2.** One opened by Joshua and reviewed by
   Jonathan without touching the branch; one opened by Jonathan and reviewed by
   Joshua, where the review question is *does it run*. The point is not the
   content. The point is that both directions have been exercised once.
6. **Write the first frontier report** (R6). One page, and the trunk stops
   having to infer intent from commits.

## 6. How you will know it is working

- In any given month, both names appear as the author of at least one merged
  PR that the other reviewed.
- No PR is merged while its author has an unanswered review request on it.
- Zero merges with a required check red, neutral or absent.
- Every row in `docs/decisions-pending/` has an `owner:` and a `rule-by:` date,
  and none is past its date.
- `MAINTAINERS` lists two people, and `CODEOWNERS` matches the lanes in §4.

None of these measure goodwill. That is the point: they are the same kind of
artefact the repository already prefers — a claim that can fail.

---

## Appendix A — reproducing the measurements

```sh
gh api 'repos/metadatastician/IDApTIK/commits?sha=main&per_page=100' --paginate   # authors, counts
gh api 'repos/metadatastician/IDApTIK/pulls?state=all&per_page=100' --paginate    # PRs, authors, merge times
gh pr view 16 --json createdAt,mergedAt,mergedBy,comments,commits                 # the case study
gh api 'search/issues?q=repo:metadatastician/IDApTIK+type:issue' --jq .total_count
cat .github/MAINTAINERS .github/CODEOWNERS docs/PROJECT-CONTRIBUTION-PROFILE.adoc
grep -rn 'developer@joshuajewell.dev\|jonathan.jewell@gmail.com' --include='*' . \
  --exclude-dir=.git
gh api repos/JoshuaJewell/IDAprUSTIK/contents/README.md --jq .content | base64 -d
sed -n '490,500p' tests/ci_security_config_test.sh        # the contact-gate comment
```

## Appendix B — proposed diffs (not applied)

Left unapplied deliberately: a governance change made by one person, on a
branch, without the other's agreement, would be an instance of the problem this
document describes rather than a fix for it.

`.github/MAINTAINERS`

```diff
 | Name | GitHub | Role | Since |
 |------|--------|------|-------|
-| Jonathan D.A. Jewell | @hyperpolymath | Primary | Project Start |
+| Jonathan D.A. Jewell | @hyperpolymath | Maintainer — frontier, estate contracts, verification, cross-repo boundaries | Project Start |
+| Joshua Jewell | @JoshuaJewell | Maintainer — trunk, player-facing surface, install/launch path, release quality | Project Start |
```

`.github/CODEOWNERS`

```diff
-# Default: sole maintainer for all files
-* @hyperpolymath
-
-# Security-sensitive files require explicit ownership
-SECURITY.md @hyperpolymath
-.github/workflows/ @hyperpolymath
-.machine_readable/ @hyperpolymath
-.machine_readable/contractiles/ @hyperpolymath
+# Default: either maintainer may approve
+* @hyperpolymath @JoshuaJewell
+
+# Frontier lane — claims, contracts, verification, estate boundaries
+crates/idaptik-ffi/ @hyperpolymath @JoshuaJewell
+crates/idaptik-kernel/ @hyperpolymath @JoshuaJewell
+contracts/ @hyperpolymath @JoshuaJewell
+verif/ @hyperpolymath @JoshuaJewell
+.github/workflows/ @hyperpolymath @JoshuaJewell
+.machine_readable/ @hyperpolymath @JoshuaJewell
+SECURITY.md @hyperpolymath @JoshuaJewell
+
+# Trunk lane — what a player or a new contributor actually touches
+install.sh @JoshuaJewell @hyperpolymath
+launcher.sh @JoshuaJewell @hyperpolymath
+justfile @JoshuaJewell @hyperpolymath
+crates/idaptik-bevy/ @JoshuaJewell @hyperpolymath
+crates/idaptik-tui/ @JoshuaJewell @hyperpolymath
+docs/MULTIPLAYER-TROUBLESHOOTING.md @JoshuaJewell @hyperpolymath
```

`docs/PROJECT-CONTRIBUTION-PROFILE.adoc`

```diff
-Contribution perimeter:: Core (single maintainer today; no Trusted-contributor or Community-sandbox tier established yet)
+Contribution perimeter:: Core (two named maintainers with scoped lanes — see `docs/dev-notes/working-agreement-2026-09-27.md` §4; no Trusted-contributor or Community-sandbox tier established yet)
```

`docs/PROJECT-ASSURANCE-PROFILE.adoc`

```diff
-Known limitations/residual risk:: No security audit; no fuzzing of the FFI surface; single maintainer means bus-factor risk on review capacity.
+Known limitations/residual risk:: No security audit; no fuzzing of the FFI surface; review capacity remains thin — two maintainers, and until the working agreement above is ratified every merged change in this repository's history was approved by at most one person.
```
