# Triage Labels

The skills speak in terms of canonical triage roles. This file maps those roles
to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker                 | Meaning                                  |
| -------------------------- | ------------------------------------ | ---------------------------------------- |
| `needs-triage`             | `state/triage`                       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `state/triage` + `needs-info`        | Waiting on reporter for more information |
| `ready-for-agent`          | `state/ready`                        | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `state/ready` + `exec/human`         | Requires human implementation            |
| `wontfix`                  | `state/dropped`, and close the issue | Will not be actioned                     |
| `bug`                      | `kind/fix`                           | Category role: something is broken       |
| `enhancement`              | `kind/feat`, or closest `kind/*`     | Category role: new work or improvement   |

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the
corresponding label strings from this table.

## Rules

- An actionable issue has exactly one `kind/*`: `feat`, `fix`, `refactor`,
  `perf`, `test`, `docs`, `build`, `ci`, `chore`, `research`, or `decision`.
- An issue has exactly one `state/*` before execution starts.
- An issue has at most one `work/*`. Use `work/*` only for non-code work; code
  execution state comes from branches and pull requests.
- Move an issue to `state/shaping` when grilling or specification starts.
- `state/triage` means the maintainer hasn't yet accepted the issue;
  `state/shaping` means they have. Add `needs-info` to a `state/triage` issue
  when the maintainer needs more from the reporter before deciding. When the
  reporter replies, remove `needs-info` so the issue is back up for evaluation.
- `state/ready` without `exec/human` means an agent may execute the issue.
- Add `role/spec` to a specification, alongside its `kind/*`. A spec is carried
  out through its child tickets, so don't pick it up as executable work.
- Preserve labels these rules don't mention.
