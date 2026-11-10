# GitHub API for Loading a Repository's Issue Graph

Research for issue #12. Question: what GitHub API surface lets Miniban load one
repository's complete issue graph (issues, labels, milestones, assignees,
sub-issues, native dependencies, open and closed) efficiently, and what limits
apply?

Sources were read on 2026-11-09. Live probes ran read-only with `gh api` against
`miniban-dev/miniban` (issues #11 to #31, which use sub-issues and native
dependencies). Facts marked "Probed" were observed on the live API. Sections
headed "Inference" are reasoning, not documented behavior.

## Summary

Both REST and GraphQL expose sub-issues and dependencies, and both are GA.
GraphQL loads the whole graph for a few hundred issues in one page per 100
issues, at about 3 to 5 points per page. REST also lists parent links and
relationship counts in the issue list, but it needs one extra request per issue
that has dependencies. Only REST supports conditional requests, and a 304 does
not cost primary rate limit. Relationship changes do NOT bump an issue's
`updated_at`, so `since` filters miss them. The repository issue events feed
does report them.

## REST versus GraphQL Coverage

### Verified facts

The REST list endpoint `GET /repos/{owner}/{repo}/issues` takes `state=all`,
`since`, `sort`, `per_page` (max 100), and filters for `labels`, `milestone`,
`assignee`, and `type`. It also returns pull requests, which carry a
`pull_request` key.
<https://docs.github.com/en/rest/issues/issues#list-repository-issues>

Probed: each issue object in that list includes `labels`, `assignees`,
`milestone`, `type`, `state_reason`, `parent_issue_url`, `sub_issues_summary`
(total, completed), and `issue_dependencies_summary` (`blocked_by`, `blocking`,
and totals). The list does NOT include the IDs of the blocked-by or blocking
issues.

The REST sub-issue endpoints, all under
`/repos/{owner}/{repo}/issues/{issue_number}`, are: `GET /parent`, `GET
/sub_issues` (paginated, max 100), `POST /sub_issues` (`sub_issue_id`, optional
`replace_parent`), `DELETE /sub_issue`, and `PATCH /sub_issues/priority`
(`after_id` or `before_id`). <https://docs.github.com/en/rest/issues/sub-issues>

The REST dependency endpoints, under the same prefix, are: `GET
/dependencies/blocked_by`, `GET /dependencies/blocking` (paginated, max 100),
`POST /dependencies/blocked_by` (body `issue_id`), and `DELETE
/dependencies/blocked_by/{issue_id}`. Only the blocked-by side is writable.
These endpoints take the issue's database `id`, not its number.
<https://docs.github.com/en/rest/issues/issue-dependencies>

Probed (GraphQL introspection): the `Issue` type has `parent`, `subIssues`,
`subIssuesSummary`, `blockedBy`, `blocking`, `issueDependenciesSummary`,
`issueType`, `relatesTo`, and `issueFieldValues`. `Repository.issues` takes
`states` and `filterBy: IssueFilters`, which includes `since`, `labels`,
`milestoneNumber`, `assignee`, and `type`.

Probed: one GraphQL query for 100 issues, with labels, assignees, milestone,
type, parent, `subIssues(first: 50)`, `blockedBy(first: 20)`, and
`blocking(first: 20)`, cost 5 points (nodeCount 12,100). A leaner query (parent
and `blockedBy(first: 50)` only, deriving children and blocking edges from
those) cost 3 points.

### Limits on the relationships

- Up to 100 sub-issues per parent, nested up to eight levels. Sub-issues can
  come from other repositories.
  <https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/adding-sub-issues>
- Up to 50 issues per dependency relationship type. Dependencies became GA on
  2025-08-21, with REST, GraphQL, and webhook support.
  <https://github.blog/changelog/2025-08-21-dependencies-on-issues/>

### Request counts for about 300 issues

Verified inputs are above. The arithmetic is inference.

- GraphQL: 3 paginated queries (100 issues each) load every node and edge. About
  9 to 15 points of the 5,000 per hour.
- REST: 3 list pages give issues, labels, assignees, milestones, types, and
  parent edges. Dependency edges need one `GET .../dependencies/blocked_by` per
  issue whose `issue_dependencies_summary.blocked_by` is non-zero. That is up to
  300 more requests, and 0 when no issue has blockers. Repository labels and
  milestones are one list call each.

## Pagination and Rate Limits

### Verified facts

REST pagination follows the `link` header. `per_page` is capped at 100, and
larger values are silently reduced.
<https://docs.github.com/en/rest/using-the-rest-api/using-pagination-in-the-rest-api>

GraphQL connections require `first` or `last`, from 1 to 100, and use cursors
(`pageInfo.endCursor`). One call may request at most 500,000 nodes. Cost is the
number of connection requests, assuming each hits its `first` maximum, divided
by 100 and rounded, with a minimum of 1. Requests over 10 seconds are terminated
with a 502 or 504, and the timeout costs extra points.
<https://docs.github.com/en/graphql/overview/rate-limits-and-query-limits-for-the-graphql-api>

Primary limits for a user token: REST 5,000 requests per hour, GraphQL 5,000
points per hour. They are tracked as separate buckets (probed: the `core` and
`graphql` resources in `GET /rate_limit`).
<https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api>

Secondary limits:

- 100 concurrent requests, shared by REST and GraphQL.
- REST: 900 points per minute. Most GETs cost 1 point and most writes cost 5.
- GraphQL: 2,000 points per minute. A query costs 1 and a mutation costs 5.
- Content creation: 80 requests per minute and 500 per hour. Web UI actions
  count too. The sub-issue and dependency write endpoints warn that they can
  trigger secondary limits.
- GitHub advises making requests serially, through a queue.
  <https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api>

## Conditional Requests

### Verified facts

Most REST endpoints return an `etag` header and many return `last-modified`.
Send them back as `if-none-match` or `if-modified-since`. An authenticated
request that gets a `304` does not count against the primary rate limit.
<https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api>

Probed: `GET /repos/miniban-dev/miniban/issues?state=all&per_page=100` returned
a weak ETag. Repeating it with `If-None-Match` twice returned `304`, and
`X-RateLimit-Used` did not change. Single issues and the repository issue events
list also return ETags. Responses carry `Cache-Control: private, max-age=60`.

Probed: GraphQL responses return no `etag` or `last-modified` header. The
GraphQL docs do not mention conditional requests.

## Detecting Change Cheaply

### Verified facts

REST `since` and GraphQL `filterBy.since` filter on the issue's last update time
(`updated_at`), not its creation time.
<https://docs.github.com/en/rest/issues/issues#list-repository-issues>

Probed: relationship changes do NOT update `updated_at`. Issue #23 has
`updated_at` equal to `created_at` (04:44:18Z), though its timeline shows
`parent_issue_added` at 04:44:20Z and `blocked_by_added` at 04:45:04Z. Issue
#11's `updated_at` (04:42:39Z) predates all 20 of its `sub_issue_added` events
(04:43:50Z onward). Assignment did bump it: #12's `updated_at` matches its
`assigned` event.

Probed: the repository issue events endpoint `GET
/repos/{owner}/{repo}/issues/events` returns relationship events
(`sub_issue_added`, `parent_issue_added`, `blocked_by_added`, `blocking_added`)
alongside `labeled`, `assigned`, and others, and it supports ETags. The docs
list sub-issue, parent-issue, blocked-by, and blocking added and removed event
types. <https://docs.github.com/en/rest/issues/events>

Webhooks: `issues` covers opened, edited, closed, reopened, labeled, unlabeled,
assigned, milestoned, typed, untyped, transferred, deleted, and more.
`sub_issues` covers parent and sub-issue added and removed. `issue_dependencies`
covers blocked-by and blocking added and removed. GitHub prefers webhooks to
polling, and polling should honor `x-poll-interval` when present.
<https://docs.github.com/en/webhooks/webhook-events-and-payloads>

### Inference

The REST issue list includes relationship summaries, so a relationship change
probably changes that page's body and its ETag. This was not tested, because it
needs a write. A summary count change does not show which edge changed.

Webhooks need a publicly reachable endpoint. A loopback-only tool cannot receive
them without a relay or tunnel, which is close to the hosted deployment that is
out of scope.

## Write Endpoints

### Verified facts

Labels can be added and removed incrementally: `POST /issues/{n}/labels`
appends, `DELETE /issues/{n}/labels/{name}` removes one (404 if absent), `PUT
/issues/{n}/labels` replaces all, and `DELETE /issues/{n}/labels` clears all.
<https://docs.github.com/en/rest/issues/labels>

`PATCH /issues/{n}` replaces labels and assignees wholesale and sets `state`,
`state_reason`, `milestone`, and `type`. Without push access, changes to labels,
assignees, milestone, and type are silently dropped.
<https://docs.github.com/en/rest/issues/issues#update-an-issue>

Probed (GraphQL introspection): mutations include `addLabelsToLabelable`,
`removeLabelsFromLabelable`, `clearLabelsFromLabelable`,
`addAssigneesToAssignable`, `removeAssigneesFromAssignable`, `addSubIssue`,
`removeSubIssue`, `reprioritizeSubIssue`, `addBlockedBy`, `removeBlockedBy`,
`updateIssueIssueType`, `updateIssue`, `closeIssue`, and `reopenIssue`.

## Issue Types and Other Recent Features

### Verified facts

Issue types are defined per organization, up to 25. The defaults are Task, Bug,
and Feature. The docs only cover organizations.
<https://docs.github.com/en/issues/tracking-your-work-with-issues/configuring-issues/managing-issue-types-in-an-organization>
Probed: `miniban-dev` has Task, Bug, and Feature enabled, and no issue in the
repository uses a type.

"Relates to" is a non-directional issue relationship. It entered public preview
on 2026-08-07, and a later changelog headline (2026-09-25) calls it generally
available. Probed: GraphQL `Issue.relatesTo` exists.
<https://github.blog/changelog/2026-08-07-connecting-issues-and-multi-select-field-support/>

Issue fields (structured custom metadata per organization) became GA on
2026-07-02. They appear as `issue_field_values` in REST and `issueFieldValues`
in GraphQL.
<https://github.blog/changelog/2026-07-02-issue-fields-are-now-generally-available/>

REST API version `2026-03-10` is the first calendar version with breaking
changes. It removes the singular `assignee` field from issues, and the `rate`
property from `GET /rate_limit`. Requests without `X-GitHub-Api-Version` still
default to `2022-11-28`, which stays supported for at least 24 months.
<https://github.blog/changelog/2026-03-12-rest-api-version-2026-03-10-is-now-available/>

The `gh` CLI manages sub-issues, types, and dependencies from v2.94.0.
<https://github.blog/changelog/2026-06-10-manage-sub-issues-types-and-dependencies-from-github-cli/>

## Implications for the Decisions

### Provider abstraction (section 14)

- Model relationships as edges: parent and child, and blocked-by (blocking is
  its inverse). Edges can point to issues in other repositories, so an edge
  target needs a repository as well as a number.
- Writes are incremental at the API: add or remove one label, assignee,
  sub-issue, or blocker. The provider interface can expose add and remove
  operations rather than replace-all, which avoids lost updates.
- GitHub keys sub-issue and dependency writes by the issue's database `id` or
  node ID, not its number. The provider has to map between them.
- Issue types, issue fields, and "relates to" are optional, org-level features.
  Treat them as provider capabilities, not core domain fields, so Gitea and
  native backends are not ruled out.
- Limits worth surfacing as validation: 100 children per parent, 8 levels of
  nesting, and 50 links per dependency type.

### Data loading and freshness (section 16, no local database)

- Full load: GraphQL, about 3 to 5 points per 100 issues, gives the whole graph
  in a few round trips. That makes a full reload on demand affordable.
- Cheap polling: REST conditional requests cost nothing on a 304. GraphQL has no
  conditional requests.
- Do not rely on `updated_at` or `since` alone. They miss relationship changes.
  The repository issue events feed (REST, ETag-capable) reports relationship,
  label, and assignment changes, so it can serve as a change signal that
  triggers a reload.
- Webhooks are the documented ideal, but they do not reach a loopback-only tool
  without a relay.
- Make requests serially and stay under 80 writes per minute, especially for
  bulk agent-driven updates.
