# Discovering GitHub issue, branch, and pull request associations

Research for issue #13 (map #11). Question: how can Miniban reliably discover,
from GitHub alone, which branches and pull requests are associated with an
issue, and what does each method miss?

Researched 2026-11-09 against docs.github.com, the GitHub changelog, the `gh`
manual, and read-only probes of the live GraphQL API against
`miniban-dev/miniban`. The live repository is small (one merged PR, #10, which
closed #2; tickets #3-#8 were committed directly on the PR branch and closed by
hand), so the probes confirm field shapes and a few behaviours, not statistics.

## Summary

The only association that is first-class, bulk-queryable, survives branch
deletion, and is produced by ordinary agent workflows is the **closing
reference** between a pull request and an issue (`closingIssuesReferences` /
`closedByPullRequestsReferences`). It comes from either a closing keyword in the
PR body or a manual link in the Development sidebar. Everything else is either
rare in practice (linked branches), transient (linked branches are hidden once a
PR exists), or a weaker signal that needs interpretation (timeline references,
commit messages, branch names).

Closing references have two important blind spots. Keywords only count when the
PR targets the default branch, and a PR can close many issues but only the
issues it names. Work landed on a shared branch for several tickets (as with
#3-#8 on `chore/scaffold`) has no closing reference; it is only reachable
through commit `ReferencedEvent`s whose commits resolve to the PR through
`Commit.associatedPullRequests`.

## Method 1: Linked branches

### Verified facts

- GitHub added `Issue.linkedBranches`, `createLinkedBranch`, and
  `deleteLinkedBranch` to GraphQL on 2022-10-31. Source:
  https://github.blog/changelog/2022-10-31-graphql-apis-for-creating-a-branch-linked-to-an-issue
- The UI feature ("Development" sidebar, create a branch from an issue) shipped
  as public beta on 2022-03-01, and the docs still say it "is currently in
  public preview and subject to change". Sources:
  https://github.blog/changelog/2022-03-01-create-a-branch-for-an-issue and
  https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-a-branch-for-an-issue
- `LinkedBranch` has only `id` and `ref`; `ref` is a nullable `Ref` (confirmed
  by introspection on the live API). Source:
  https://docs.github.com/en/graphql/reference/issues
- `createLinkedBranch` creates a new branch; its `name` input "Defaults to issue
  number and title". There is no mutation to link an existing branch. Source:
  https://docs.github.com/en/graphql/reference/issues
- Creating a PR from a linked branch links the PR to the issue automatically;
  "The connection with that branch is removed and only the pull request is shown
  in the Development section." Source:
  https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-a-branch-for-an-issue
- `gh issue develop` creates (and with `--list` lists) linked branches. It is
  the only `gh` path that produces a linked branch; `git push` and `gh pr
  create` do not. Source: https://cli.github.com/manual/gh_issue_develop
- There is no REST endpoint for linked branches in the issues reference; the
  feature is GraphQL-only. (Not exhaustively verified; no such endpoint was
  found in the REST issues docs.)
- Live probe: all 21 issues in the repo return `linkedBranches.totalCount: 0`.

### Inference

- Whether `linkedBranches` still returns the branch after a PR is opened (the UI
  hides it) and what happens when the branch is deleted (null `ref` or removed
  link) was not verified; testing would need writes. The nullable `ref` suggests
  a deleted branch can leave a dangling link.
- Agents and humans using plain `git checkout -b` + `git push` never produce
  this association, so it will be absent for most work unless Miniban (or the
  agent skill) calls `createLinkedBranch` / `gh issue develop` itself.

## Method 2: Closing references (keywords and manual links)

### Verified facts

- Nine keywords (close/closes/closed, fix/fixes/fixed, resolve/resolves/
  resolved) link a PR to an issue, but only when "The pull request must be on
  the default branch" (i.e. targets it). Cross-repo form is `Fixes
  owner/repo#100`. Manual links via the sidebar are capped at ten issues per PR.
  Keyword links can only be removed by editing the PR body. Source:
  https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/linking-a-pull-request-to-an-issue
- A closing keyword in a commit message closes the issue when merged to the
  default branch, but the PR containing it "isn't listed as a linked pull
  request". Source: same as above.
- `PullRequest.closingIssuesReferences` ("issues that may be closed by this pull
  request") has `userLinkedOnly`, `excludeUserLinked`, `orderBy`. Source:
  https://docs.github.com/en/graphql/reference/pulls
- `Issue.closedByPullRequestsReferences` has `includeClosedPrs` (default false),
  `orderByState`, `userLinkedOnly`, `excludeUserLinked`. Source:
  https://docs.github.com/en/graphql/reference/issues
- Live probe: `closedByPullRequestsReferences(first: 10)` on #2 with default
  arguments returns PR #10 in state `MERGED`. So `includeClosedPrs: false` still
  includes merged PRs; it only hides closed-unmerged PRs. (A third-party claim
  that the default hides merged PRs is contradicted by this probe.)
- Live probe: PR #10's head branch `chore/scaffold` was deleted
  (`HeadRefDeletedEvent`); `headRef` is null but `headRefName` is still
  `chore/scaffold`, and the closing reference to #2 is intact. `headRefName` is
  documented as the name "even if the ref has been deleted". Source:
  https://docs.github.com/en/graphql/reference/pulls
- Live probe: `userLinkedOnly: true` returns 0 for #2, confirming the link came
  from the `Closes #2` keyword in the PR body.
- Search supports `linked:pr` / `-linked:pr` (issues) and `linked:issue` (PRs),
  matching closing references only. Live probe: `repo:miniban-dev/miniban
  is:issue linked:pr` returns #2 (closed issue, so the qualifier also matches
  after merge). Source:
  https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests

### Inference

- Draft PRs: not covered in docs and not probed (no drafts exist). `isDraft` is
  available on each node, and nothing in the schema suggests drafts are
  excluded, so drafts very likely appear like open PRs.
- This is the association agents most often produce: `gh pr create --body` with
  `Closes #N` is the standard convention, and this repo's PR template already
  does it.

## Method 3: Timeline events

### Verified facts

- `connected` / `disconnected`: "The issue or pull request was linked to /
  unlinked from another issue or pull request" (the manual Development link).
  Available in both the issue events and timeline REST APIs. Source:
  https://docs.github.com/en/rest/using-the-rest-api/issue-event-types
- `cross-referenced`: referenced from another issue or PR; timeline API only
  (not the issue events API). GraphQL `CrossReferencedEvent.willCloseTarget`
  "Checks if the target will be closed when the source is merged." Sources:
  https://docs.github.com/en/rest/using-the-rest-api/issue-event-types and
  https://docs.github.com/en/graphql/reference/issues
- `referenced`: "The issue was referenced from a commit message", with
  `commit_id`. `closed` carries `commit_id` when a commit closed the issue.
  Source: https://docs.github.com/en/rest/using-the-rest-api/issue-event-types
- Live probe: #2 has a `CrossReferencedEvent` from PR #10 and a `ClosedEvent`
  whose `closer` is PR #10. `willCloseTarget` reads `false` after the merge, so
  it reflects current state, not history.
- Live probe: #3 has `ReferencedEvent`s for branch commit `33b3a9b` and the
  squash commit `6bc4777`; both resolve through `Commit.associatedPullRequests`
  to PR #10 (`chore/scaffold`). Its `ClosedEvent.closer` is null (closed by
  hand). This is the only API path that ties #3-#8 to PR #10.
- Live probe: cross-references also come from issues (#1 from #2 and #11, #2
  from #3-#9), so filtering by `source.__typename == PullRequest` is needed.

### Inference

- Timelines are per-issue connections, so bulk use is one nested connection per
  issue; affordable at small scale (see cost below) but noisy: any mention
  creates a cross-reference, so "mentioned" is not "worked on".

## Method 4: Branch naming conventions

### Verified facts

- `createLinkedBranch` / `gh issue develop` default names use the issue number
  and title (doc wording above); the common observed pattern is `123-some-slug`.
  Source: https://docs.github.com/en/graphql/reference/issues
- Branches can be listed in bulk with `repository.refs(refPrefix:
  "refs/heads/")`, each with `associatedPullRequests`. Search supports `head:`
  (prefix match) for PRs. Source:
  https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests
- Live probe: this repo's branches (`chore/scaffold`, `research/<name>`) carry
  no issue number, and deleted branches disappear from `refs` (only `main`
  remains); `headRefName` on PRs keeps the name.

### Inference

- A fallback only: it misses every branch that does not follow the convention
  (all current branches here), gives false positives on numbers in names, and
  says nothing once the branch is deleted unless read from a PR's `headRefName`.

## Comparison

| Method                         | Bulk query                                                   | Draft PR                        | Closed unmerged              | Merged               | Deleted branch             | Cost     | Produced by real workflows                    |
| ------------------------------ | ------------------------------------------------------------ | ------------------------------- | ---------------------------- | -------------------- | -------------------------- | -------- | --------------------------------------------- |
| Linked branches                | Yes, nested on `issues`                                      | n/a (hidden once PR exists, UI) | n/a                          | n/a                  | Unverified; `ref` nullable | Low      | Rarely; only `gh issue develop` or UI         |
| Closing refs                   | Yes, from `issues` or `pullRequests`; also `linked:` search  | Likely yes (`isDraft`)          | Only with `includeClosedPrs` | Yes by default       | Yes; `headRefName` kept    | Low      | Common (`Closes #N`), default-branch PRs only |
| Timeline cross-ref / connected | Per issue, nested                                            | Yes (any mention)               | Yes                          | Yes                  | Yes                        | Moderate | Common but noisy                              |
| Commit referenced events       | Per issue, nested; commit to PR via `associatedPullRequests` | Yes                             | Yes                          | Yes                  | Yes                        | Moderate | Common when commits cite `#N`                 |
| Branch name                    | `refs` list, `head:` search                                  | Via PR                          | Via PR                       | Via PR `headRefName` | Lost from `refs`           | Low      | Convention-dependent; none here               |

Cost: a single query for 100 issues (each with `linkedBranches` and
`closedByPullRequestsReferences`) plus 100 PRs (each with
`closingIssuesReferences` and 100 commits) cost 4 points against the
5,000-points-per-hour user budget; `first`/`last` max is 100 and a call is
capped at 500,000 nodes. Source:
https://docs.github.com/en/graphql/overview/rate-limits-and-query-limits-for-the-graphql-api

## Implications for the decisions

- Treat **closing references** as the primary, authoritative "associated PR"
  signal for execution state: read `closedByPullRequestsReferences(
  includeClosedPrs: true, orderByState: true)` per issue in the bulk issues
  query, and use PR `state`, `isDraft`, and `headRefName` to derive states such
  as in progress, in review, and merged.
- Define the **associated working branch** as the head branch of an associated
  PR (via `headRefName`), plus any `linkedBranches`. Branch-only work with no PR
  is discoverable only if it is a linked branch or follows a naming convention;
  decide whether Miniban should create linked branches itself (via
  `createLinkedBranch`) when work starts so that pre-PR work is visible.
- Child tickets landed on a shared branch (the scaffold pattern) have no closing
  reference. Either accept that the parent PR stands for them, or add a
  secondary signal: commit `ReferencedEvent`s resolved via
  `associatedPullRequests`. That costs a timeline connection per issue.
- PRs that target a non-default base (stacked work) never form keyword links;
  they need manual links or the commit-reference fallback.
- Treat timeline cross-references and branch names as hints, not state: mentions
  are noisy and this repo's branch names carry no issue numbers.
- All of this fits in a few GraphQL points per refresh for a repository of this
  size, so no local database is needed for association discovery.
