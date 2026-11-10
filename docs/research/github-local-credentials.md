# GitHub credentials for a local single-user tool

Research for issue #14. Question: how can a local, single-user `miniban` server
on loopback get GitHub credentials, and what does each option need?

Sources were read on 2026-11-09. The gh CLI source was read at commit
`18138654f3598ebe031ad7b661c903188f0f1dc9` of `cli/cli`.

## Access Miniban needs

Read and write issues, labels, milestones, sub-issues, and issue dependencies.
Read branches and pull requests. Act as the user, on repositories the user
picks, which may be user-owned or organization-owned.

## Verified facts

### Fine-grained permissions per endpoint

Source:
[Permissions required for fine-grained personal access tokens][fg-perms]. The
same permission names apply to GitHub App permissions.

| Operation                               | Permission    | Level |
| --------------------------------------- | ------------- | ----- |
| Create or update an issue               | Issues        | write |
| Add, set, or remove labels on an issue  | Issues        | write |
| Create a label                          | Issues        | write |
| Create or update a milestone            | Issues        | write |
| List sub-issues                         | Issues        | read  |
| Add, remove, or reprioritize sub-issues | Issues        | write |
| List `blocked_by` dependencies          | Issues        | read  |
| Add or remove `blocked_by` dependencies | Issues        | write |
| List branches                           | Contents      | read  |
| List or get pull requests               | Pull requests | read  |

Label, milestone, and issue-update endpoints accept either "Issues" or "Pull
requests" write. Sub-issue and dependency endpoints need "Issues" only.

So the full fine-grained set is: Issues read/write, Contents read, Pull requests
read (plus Metadata read, see Inference).

### Classic scopes

Source: [Scopes for OAuth apps][scopes].

- There is no issues-only scope. `repo` "Grants full access to public and
  private repositories", including code. `public_repo` limits that to public
  repositories.
- OAuth scopes cannot be limited to specific repositories.

### Fine-grained personal access token (PAT)

Source: [Managing your personal access tokens][pat].

- Limited to one resource owner (a user or one organization) and optionally to
  selected repositories. GitHub recommends it over classic tokens.
- Expiry: 1 to 366 days or none; default 30 days. Organizations and enterprises
  can impose a maximum lifetime. No refresh: the user makes a new token when it
  expires.
- Organizations can require owner approval. A pending token can only read public
  resources until approved.
- One token cannot cover several organizations at once. Not supported for
  repositories where the user is an outside collaborator, or for contributing to
  public repositories the user is not a member of.
- SAML SSO: "Fine-grained personal access tokens are authorized during token
  creation" ([SSO PAT authorization][sso-pat]).

### Classic personal access token

Sources: [pat], [sso-pat].

- Uses scopes; `repo` is needed for private repositories, and covers every
  repository the user can reach.
- Organizations can block classic tokens; requests then get 403.
- For SAML SSO organizations, the user must authorize the token for SSO after
  creation.

### Reusing the gh CLI token

Sources: [gh auth token][gh-token], [gh auth login][gh-login],
[gh environment][gh-env], [gh source][gh-src].

- `gh auth token` prints the token for the active account on a host. If the
  token is expired or near expiry and refreshable, it is refreshed before
  printing unless `--no-refresh` is passed.
- `gh auth login` stores the token in the system credential store, and falls
  back to a plain text file when the store is unavailable, or always with
  `--insecure-storage`.
- In source, the keyring service name is `"gh:" + hostname`
  (`internal/config/config.go`), using `zalando/go-keyring`. The plain text
  fallback is the `oauth_token` key in the gh config (`hosts.yml`).
- gh now also keeps a separate `refreshable_oauth_token` record for expiring,
  refreshable credentials, with a cross-process lock because the refresh token
  is single use (`internal/config/refreshable.go`).
- The gh login flow is an OAuth flow with gh's own client ID, requesting the
  minimum scopes `repo`, `read:org`, `gist` (`internal/authflow/flow.go`).
- `GH_TOKEN`, then `GITHUB_TOKEN`, take precedence over stored credentials for
  github.com.

### OAuth app with device flow

Source: [Authorizing OAuth apps][oauth].

- Device flow must be enabled in the app settings. Request codes at `POST
  https://github.com/login/device/code` with `client_id` and `scope`; the user
  enters the code at `https://github.com/login/device`; the app polls `POST
  https://github.com/login/oauth/access_token`.
- "The `client_secret` is not needed for the device flow." A public client can
  ship only a client ID.
- The user has 15 minutes to enter the code. Polling faster than `interval`
  returns `slow_down`, which adds 5 seconds.
- Tokens are long-lived by default. With expiring tokens enabled the access
  token lasts 8 hours and the refresh token 6 months; refresh tokens are single
  use.
- The web flow supports loopback redirect URIs on any port (prefer `127.0.0.1`
  or `::1`) and PKCE with `S256` only.
- Permissions are classic scopes (see above), so writing issues on private
  repositories needs `repo`.
- Organizations have OAuth app access restrictions, on by default for new
  organizations. Unapproved OAuth apps cannot reach private organization
  resources ([OAuth app access restrictions][oauth-restrict]).

### GitHub App user access token

Sources: [Generating a user access token][app-uat],
[Authorizing GitHub Apps][app-authz].

- Web flow (with PKCE) or device flow; device flow must be enabled in the app
  settings, and no client secret is listed for it.
- "By default, the user access token expires after 8 hours." A refresh token
  lasts about 6 months. Expiring tokens can be turned off.
- "A GitHub App user access token only has permissions that both the user and
  the app have." Repository access is the same intersection, so the app must be
  installed on the repository or organization for the token to reach it.
- Installing (grants repository access) and authorizing (acts as the user) are
  separate steps.
- SAML SSO: the user needs an active SSO session when authorizing an OAuth app
  or GitHub App, and must reauthorize after SSO is enforced ([SSO][sso]).

## Comparison

Access, setup, and lifetime:

| Option           | Access             | User setup    | Lifetime        |
| ---------------- | ------------------ | ------------- | --------------- |
| Fine-grained PAT | Chosen, 3 perms    | Web UI form   | <=366 d or none |
| Classic PAT      | All, `repo`        | Web UI, SSO   | Chosen or none  |
| gh CLI token     | All, `repo`+more   | gh login      | gh refreshes    |
| OAuth device     | All, `repo`        | Enter code    | Long, or 8 h    |
| GitHub App UAT   | Installed, 3 perms | Install, code | 8 h + refresh   |

Storage, refresh, and organization limits:

| Option           | Refresh        | Secret held by  | Organization limits |
| ---------------- | -------------- | --------------- | ------------------- |
| Fine-grained PAT | None, recreate | Miniban or env  | Approval, max life  |
| Classic PAT      | None, recreate | Miniban or env  | Blockable; SSO      |
| gh CLI token     | By gh          | gh keyring/file | gh app approval     |
| OAuth device     | 6 months       | Miniban         | OAuth restrictions  |
| GitHub App UAT   | 6 months       | Miniban         | Install; SSO        |

"3 perms" is Issues read/write, Contents read, Pull requests read.

## Inference

These are conclusions drawn from the facts above, not stated by a source.

- Metadata read is implicitly required by every fine-grained token and GitHub
  App; GitHub adds it automatically. Not checked on a source page here.
- The gh CLI token is the broadest option: `repo`, `read:org`, and `gist` over
  every repository the user can reach. Reusing it gives Miniban far more power
  than it needs, and organizations with OAuth app access restrictions must have
  approved the GitHub CLI app.
- Reading gh's keyring entry or `hosts.yml` directly couples Miniban to gh
  internals that are changing (the new refreshable record). If Miniban reuses gh
  at all, running `gh auth token` as a subprocess is the stable interface.
- A Miniban OAuth app adds a registration that Miniban maintainers own, but
  still yields `repo`-scoped tokens, so it gains little over a classic PAT in
  least privilege.
- A Miniban GitHub App gives the least privilege with no copy-paste of secrets,
  but requires the project to register and maintain a public app, and each
  organization owner to install it. Self-hosters can register their own app with
  the same client code.
- A fine-grained PAT matches the needed permission set exactly, needs no
  Miniban-owned registration, and works offline from any app approval, but has
  no refresh and covers one owner per token.
- Whatever the source, Miniban on loopback should keep the token out of the
  browser: the server holds it and the web UI never sees it.

## Implications for the decisions

Credentials: support a fine-grained PAT as the primary path (Issues read/write,
Contents read, Pull requests read), read from an env var or the OS keyring,
never from the web UI. Offer `gh auth token` as an opt-in convenience with a
clear warning about its broad scope. Keep a GitHub App device flow as a later
upgrade that the credential seam must not rule out (expiring tokens, refresh,
one token per user rather than per owner).

Project configuration: because a fine-grained PAT covers one resource owner,
configuration should map each repository to a credential, or at least detect and
report a token that cannot see a configured repository. Configuration should
name a credential source (env var name, keyring entry, or gh), not contain the
secret.

[fg-perms]: https://docs.github.com/en/rest/authentication/permissions-required-for-fine-grained-personal-access-tokens
[scopes]: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/scopes-for-oauth-apps
[pat]: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens
[sso-pat]: https://docs.github.com/en/authentication/authenticating-with-saml-single-sign-on/authorizing-a-personal-access-token-for-use-with-saml-single-sign-on
[sso]: https://docs.github.com/en/authentication/authenticating-with-saml-single-sign-on/about-authentication-with-saml-single-sign-on
[gh-token]: https://cli.github.com/manual/gh_auth_token
[gh-login]: https://cli.github.com/manual/gh_auth_login
[gh-env]: https://cli.github.com/manual/gh_help_environment
[gh-src]: https://github.com/cli/cli/tree/18138654f3598ebe031ad7b661c903188f0f1dc9
[oauth]: https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps
[oauth-restrict]: https://docs.github.com/en/organizations/managing-oauth-access-to-your-organizations-data/about-oauth-app-access-restrictions
[app-uat]: https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app
[app-authz]: https://docs.github.com/en/apps/using-github-apps/authorizing-github-apps
