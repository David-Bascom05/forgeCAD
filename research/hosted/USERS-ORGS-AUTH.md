# Users, organisations and authentication: plan

Status: **design only, no implementation**. Written 2026-11-25 by HOSTED-DESIGN for the
coordinator. Sources: `PRIOR-ART-AUTH.md` (written "A§n"; fetched 2026-11-25) and
`PRIOR-ART-PHASE2.md` ("P2§n"). Measurements: DESIGN.md §17 and MEASUREMENTS.md. Where neither
decides, **UNKNOWN** says how to find out. Some quotes in A§ are the fetch tool's excerpts; each
cited line names its URL there.

This plan fits into DESIGN.md: one Postgres store (§5, §17.1), a fenced append that checks the
author's role at commit (§17.1), LOCAL projects untouched (§7, §15), and agents through the user's
own Claude Code (§9).

## 0. Decisions in one table

| # | choice | decision | the evidence that decides it |
|---|---|---|---|
| 1 | account key | **(issuer, subject) for federated logins; an internal id for every account; email is an attribute, never a key** | OIDC Core: "sub and iss … are the only Claims that an RP can rely upon as a stable identifier"; email "MUST NOT be used as unique identifiers" (A§2) |
| 2 | sign-in methods | **passkeys first; email + password allowed (≥ 15 chars when the only factor); "sign in with Google / Microsoft" (OIDC)**; no magic links | NIST SP 800-63B rev 4: ≥ 15 characters single-factor, a phishing-resistant option SHALL be offered at AAL2, syncable authenticators OK up to AAL2 (P2§5); WebAuthn L3 is a W3C Recommendation since 25 Aug 2026 (A§8) |
| 3 | password storage | **Argon2id m=19 MiB, t=2, p=1**; ≤ 4 concurrent hashes | OWASP (P2§5); 17.8 ms p50 per hash measured here (§17.4) |
| 4 | who issues tokens | **forgeCAD runs its own authorization server** (authorization code + PKCE only), federating upstream identity providers | MCP requires an OAuth resource server with RFC 9728 metadata and audience-bound tokens (A§3); the desktop needs RFC 8252's flow (A§1); both are a small, enumerable surface (§4) |
| 5 | token type | **opaque tokens, looked up by SHA-256; no JWTs** | a JWT cannot be revoked before `exp` without "(currently non-standardized) backend interaction" (RFC 7009, A§4); we are the only resource server, so the lookup a JWT avoids costs one index read per connection, not per op |
| 6 | browser session | **`__Host-` cookie, HttpOnly, Secure, SameSite=Lax; idle 30 min sliding, absolute 12 h** (re-auth by passkey is one touch) | OWASP Session Management: ≥ 64 bits, HttpOnly, Secure, SameSite, `__Host-` prefix, server-side timeouts, no tokens in web storage (A§4); 12 h is above OWASP's 4–8 h example for a full-day office workflow — chosen for long CAD sessions with a passkey re-auth, **UNKNOWN** whether users accept 8 h instead |
| 7 | desktop app | **code + PKCE in the system browser, loopback redirect to 127.0.0.1 any port; refresh-token rotation; tokens in the OS keychain** | RFC 8252 MUST external user-agent, MUST NOT embedded webviews, loopback "any port", `localhost` NOT RECOMMENDED; RFC 9700: public clients' refresh tokens MUST be sender-constrained or rotated (A§1, A§4) |
| 8 | agents (MCP) | **the hosted MCP endpoint is an OAuth resource server per MCP 2026-07-28**; client registration by Client ID Metadata Document, DCR only as the deprecated fallback; scopes per project; agent edits attributed | MCP authorization spec 2026-07-28 (A§3) |
| 9 | orgs | **org → teams → projects → documents**; an org owns projects; a person may belong to several orgs | Onshape (enterprise, teams, projects), Figma (org, teams, folders), GitHub (org, teams, repos) all have these levels (A§5) |
| 10 | roles | **project roles: viewer, commenter, editor, admin; org roles: owner, admin, member, guest; an org base role for its projects** | Onshape "Can view/Can edit", Figma "Can view/Can edit/Owner", GitHub's base permission (A§5) |
| 11 | inheritance | **documents inherit their project's grants; a document can grant more, never less; "restricted" projects instead of per-document denies** | all four products: grant more yes, less no; restriction only by a limited folder/subtree (A§5 summary table) |
| 12 | link sharing | **view-only links, off by default per org policy; no edit links** | Onshape: link shares are view-only (A§5); Figma allows edit links (A§5) — rejected: a leaked URL becomes a write credential |
| 13 | where enforced | **every check on the server; the client only hides UI** | OWASP A01: "deny by default", "enforce record ownership" (A§7) |
| 14 | edit authorization | **the author's role is checked inside the fenced append** (§17.1) | a demoted editor's in-flight op cannot commit; measured: 0 of a viewer's appends accepted (§17.1) |
| 15 | audit | **an append-only audit table, hash-chained, exported daily to object storage; the op log is the edit audit** | OWASP Logging: what to log, no secrets, tamper detection, read-only copies (A§6) |
| 16 | enterprise SSO | **SAML 2.0 and OIDC, plus SCIM 2.0 deprovisioning, in a later phase; verified domains by DNS TXT** | Figma and GitHub lead with SAML; GitHub's OIDC is Entra-only (A§2); every vendor checked puts SSO/SCIM on top plans (A§2) |
| 17 | LOCAL projects | **no accounts, no tokens, no network, no code on the edit path** | DESIGN §7 B5: 0 instructions more |

## 1. Identity

**Accounts.** `accounts(id, created_at, display_name, status)` with `identities(account, issuer,
subject, email, email_verified, created_at)`. A login finds its account by (issuer, subject),
never by email (OIDC Core §5.7, A§2). Our own password and passkey logins use issuer
`forgecad`, subject = account id. Linking a second identity to an account requires being signed
in to it. A verified email matching an existing account is *not* enough. Otherwise whoever
controls a mail domain's IdP could take over accounts.

**Sign-in methods, compared:**

| method | phishing-resistant | needs our secret storage | works for desktop / agents | decision |
|---|---|---|---|---|
| passkey (WebAuthn) | yes (origin-bound) | public key only | through the system browser (RFC 8252 flow) — sourced for macOS, inferred for Windows/Linux (A§8, UNVERIFIED there) | **first-class** |
| email + password | no | Argon2id hash | yes (same flow) | **allowed**: 15-char minimum unless a second factor; breached-password blocklist; no composition rules, no forced rotation (NIST, P2§5) |
| social OIDC (Google, Microsoft) | as strong as the provider | none | yes | **allowed** |
| email magic link | no; and the mailbox becomes the credential | link tokens | awkward on desktop | **no** |
| SAML / enterprise OIDC | per IdP | IdP config | yes | **phase C** (§10) |

**Second factor.** A passkey is one factor that satisfies AAL2's phishing-resistance requirement
by itself (NIST, P2§5). A password account may add a passkey or TOTP. Org policy can require
passkeys (§2).

**Account recovery.** A verified-email reset puts the mailbox in charge, so recovery revokes
every session and token of the account and is audited. An org can disable self-service recovery
for its members. Recovery then goes through the org admin.

## 2. Organisations, teams, roles

```
org ──< members (role: owner | admin | member | guest)
 ├──< teams ──< team_members
 ├──< projects ──< grants (to: account | team | org-base, role: viewer | commenter | editor | admin)
 │        └──< documents ──< doc_grants (grant more only)
 └──< domains (verified by DNS TXT) and policies
personal space: every account has one; its projects have no org.
```

- **Org roles.** Owner: billing, deleting the org, everything below. Admin: members, teams,
  policies, all projects. Member: what grants give. Guest: an outside account with grants on
  named projects only; never the base role, never team membership unless policy allows. This
  follows GitHub's owners/members/outside collaborators and Onshape's guests (A§5).
- **Project roles.** Viewer: open and build locally, export subject to policy. Commenter: viewer
  plus comments. Editor: send ops, create named versions. Admin: grants, settings, move/delete.
  Onshape splits rights finer, with separate Copy, Export, Share and Delete checkboxes (A§5).
  We start with four roles plus org policies for export and copy, and add checkboxes only when
  asked. GitHub's custom repository roles are Enterprise-only for the same reason (A§5).
- **Base role.** An org may set a base role (none or viewer) that every member has on every org
  project, as GitHub's base permission does (A§5). The effective role is the highest of base,
  team and direct grants, as on GitHub (A§5).
- **Inheritance.** A document has its project's grants. A document grant can add access, never
  remove it. To keep something from people who can see the project, put it in a separate
  **restricted** project with no base role, as Figma's "Limited" folders and Google's
  limited-access folders do (A§5). No product checked has per-file denies; they make "who can
  see this?" hard to answer, and §6's audit must answer it.
- **Org policies.** Base role; link sharing on/off; guests allowed; export and personal-copy
  rights (Onshape has "Export files", "Make personal copies", "Enable link sharing" as global
  permissions, A§5); passkeys required; session absolute timeout; who may create projects.

**Verified domains.** An org proves a domain with a DNS TXT record, as Figma, Atlassian and
GitHub do (A§2). With a verified domain it may (a) see and invite everyone with that domain's
verified email, (b) require SSO for them (phase C). Atlassian notifies users when their
accounts are claimed (A§2), and so do we. Public mail domains cannot be verified.

## 3. Permissions, the op log, and LOCAL vs SERVED

**Edit.** The fenced append (§17.1) commits an op only if the author's effective role on the
document is ≥ editor *at commit*. The effective role is computed from members, grants and
policies in the same statement (a view `effective_roles(account, document)`). A demoted or
removed editor's in-flight ops are refused, which closes the race between "permission changed"
and "op arrives". Cost: §8.

**Read.** Joining a document (Hello) requires ≥ viewer, checked once. Revocation must also
cut live readers. A grants change NOTIFYs the owner servers, which recompute the role for each
live connection of the affected documents. A connection whose role fell below viewer is
closed, and one that fell below editor has its pending ops refused. A missed NOTIFY costs
nothing for edits, which the append still refuses. For reads it lasts at most until the
connection's periodic revalidation (30 min, OWASP WebSocket cheat sheet, A§7).

**Attribution.** Every op stores `author = account id` and `client` (a device or agent id). An
agent's op stores the account it acts for and the agent's client id. History shows "Ada (via
agent)". Named versions written to forge-history keep the `ForgeCAD-Agent` trailer (§1, §9).

**Blobs and compute results.** They are content-addressed, so a hash is not a secret: two
projects with the same STEP share one blob. A download is authorized per request, by role on
a document that references that hash, and served by a short-lived signed URL. Knowing a hash
gives nothing. A compute cache hit still checks the requester's read access to the job's input
document.

**LOCAL projects.** Nothing here applies. No account, no token, no network; files, `F.fg.log`
and git exactly as today (§7, B5). The LOCAL edit path gains no code.

**SERVED projects on disk** (§15.3). The local copy (`.forgecad/sync/`) is readable by anyone
with the disk. Revoking a user cannot erase copies already on their machines, just as revoking
a git collaborator cannot erase their clone. This is the residual risk in §7. Org policy
"personal copies off" stops the app from making new local copies of org projects: the desktop
keeps served documents in memory and a cache encrypted under a key from the server, and drops
the key on revocation. **UNKNOWN** whether that cache encryption is worth its complexity.
Decide from customer asks.

**Moving between modes** (§15.2). Publishing a LOCAL project creates a served project in the
user's personal space or an org where they may create projects. Checkout of a served project
into a LOCAL folder needs ≥ viewer and, in an org, the personal-copy right. Detach records an
audit event.

## 4. Sessions and tokens, per client

| client | how it signs in | what it holds | lifetime and revocation |
|---|---|---|---|
| browser | our login page (passkey, password, social OIDC) | `__Host-` session cookie (opaque, 256 bits); server keeps SHA-256 | idle 30 min sliding, absolute 12 h; logout and admin revoke delete the row; the WebSocket closes when its session ends (OWASP WS, A§7) |
| desktop app | authorization code + PKCE (S256) in the **system browser**, redirect to `http://127.0.0.1:<any port>/cb` | access token (opaque, 15 min) + refresh token (rotated on every use), in the OS keychain (macOS Keychain, Windows Credential Locker, Secret Service on Linux; A§4) | a replayed old refresh token revokes the whole grant (RFC 9700 rotation, A§4); listed per device in account settings |
| CLI on a machine with no browser | device authorization grant (RFC 8628) | same as desktop | short `expires_in`, user codes rate-limited (A§1); not for machines that have a browser |
| agent via local Claude Code (§9 option 1) | the desktop's own sign-in; the MCP bridge runs over stdio, which takes "credentials from the environment" (MCP spec, A§3) | nothing new: the bridge asks the desktop for a token scoped to the project | revoking the device revokes it |
| remote MCP client (e.g. an agent with BYO key, §9 option 3) | OAuth to our authorization server; the client registered by **Client ID Metadata Document**, DCR only as the deprecated fallback; RFC 8707 `resource` = our MCP URL | access token audience-bound to our MCP resource, scopes `project:<id>:read` / `:edit` | listed under the user's connected apps; revocable; the token is accepted only by our MCP endpoint, never passed through (MCP "MUST NOT accept or transit any other tokens", A§3) |

**One authorization server, compared with the alternatives:**

| option | what it costs | what it gives | decision |
|---|---|---|---|
| our own minimal AS in forge-server | code for: authorization endpoint (code + PKCE S256 only), token endpoint (refresh rotation), revocation (RFC 7009), AS metadata (RFC 8414), protected-resource metadata (RFC 9728), CIMD fetching, `iss` in responses (RFC 9207), upstream OIDC client | one database: role checks, sessions and tokens in the same transaction as the append; no extra hop per login | **chosen** for phases A–B |
| a self-hosted open-source IdP beside it | another service to run, upgrade and back up; tokens to map into our roles | SAML, SCIM, admin UI out of the box | **revisit at phase C**, when SAML/SCIM arrive. **UNKNOWN**: the effort of our own SAML SP with a library vs running an IdP — measure with a spike |
| a managed identity service | per-MAU pricing (not fetched: **UNKNOWN**); user data at a third party | everything, fast | not chosen without prices; noted for phase C |

The implicit and password grants are not implemented (RFC 9700 forbids or discourages them,
A§1). Redirect URIs match exactly, except the loopback port (OAuth 2.1 draft, A§1).

**WebSocket handshake** (browser): the session cookie is sent on the upgrade. The server checks
`Origin` against an exact allowlist (OWASP WebSocket cheat sheet; PortSwigger frames CSWSH as
CSRF on the handshake, A§7). `SameSite=Lax` is defence in depth: browsers are expected to withhold the cookie on a
cross-site handshake (not sourced here: UNVERIFIED); the Origin check does not rely on it. Tokens never go in the URL query (OWASP, A§7). Native clients
send `Authorization: Bearer` on the upgrade.

## 5. Sharing and invites

- **Invite** by email to an org, a team or a project with a role. Stored as SHA-256 of a
  256-bit token, expiring after 7 days. Accepting requires signing in with an identity whose
  **verified** email equals the invited one. A forwarded invite therefore does not work for
  someone else.
- **Request access**: a signed-in user without access sees "ask for access", which notifies
  the project's admins. Nothing about the project's contents is shown.
- **View links**: per document or project, view-only, revocable, optional expiry. Off when org
  policy says so. No sign-in needed to view, as on Onshape (A§5). Viewers through a link can
  build and look, cannot export unless the link says so, and are anonymous in presence.
- **Transfers**: moving a document between projects needs admin on both, and the document then
  takes the target's grants, as Onshape requires (A§5). Ownership of a personal project can be
  transferred to an org.

## 6. Audit

**What** (OWASP Logging, A§6): sign-in success and failure; session and token issue, refresh
and revoke; MFA and passkey changes; recovery; every change to members, roles, grants, links,
policies and domains; exports, checkouts to LOCAL, blob downloads of imported files; admin
actions; authorization failures (a refused op records only the refusal kind, not the op).

**Fields**: when, actor account, acting client (device or agent id), source IP, org, target,
action, result. Never passwords, tokens or keys; a session appears as the hash of its id (A§6).

**Integrity**: `audit(id, at, …, prev_hash, hash)`, each row hashing the previous row. A daily
export writes the day's rows and the chain head to object storage with object lock (write-once),
so deleting or editing a row breaks the chain against the exported head.

**Retention**: one year by default (GitHub keeps 180 days; Figma does not state it, A§6),
configurable per org. Ops are never deleted (§17.1), so the edit history is its own audit:
who changed what, when, through which client.

**Access**: org admins read their org's audit; users read their own sign-ins and devices.
Reading the audit is itself audited (OWASP, A§6).

## 7. Threat model

Assets: designs (customers' IP), accounts, the integrity of the op log. Mapped to OWASP ASVS
5.0 chapters V6 Authentication, V7 Session Management, V8 Authorization, V10 OAuth/OIDC and V16
Logging (A§7), and to OWASP Top 10 A01 Broken Access Control.

| threat | example | mitigation | residual |
|---|---|---|---|
| credential phishing | fake login page | passkeys first (origin-bound); org can require them | password users remain phishable |
| credential stuffing | reused passwords | breached-password blocklist; per-account and per-IP rate limits; 15-char minimum | slow guessing of unique passwords |
| session theft | XSS reading a token | HttpOnly cookie, no tokens in web storage (OWASP, A§4); CSP | an XSS can still act *in* the page while open |
| cross-site WebSocket hijacking | evil.com opens our socket with the cookie | exact Origin allowlist; SameSite=Lax (A§7) | — |
| IDOR | asking for another project's document id | every query joins effective roles; ids never authorize (OWASP A01) | — |
| demoted editor races an op | op in flight during demotion | role checked in the commit statement (§3) | — |
| stolen desktop refresh token | malware reads the keychain | rotation: a replay revokes the grant (RFC 9700); device list; revoke | access until the next refresh (≤ 15 min) |
| malicious project file | `.forgecad/remote.toml` naming an attacker's server | already closed: a project names a remote only from the user's own `remotes.toml`, with listed project folders (reviews N1-N9) | — |
| compromised or prompt-injected agent | an agent told to delete or exfiltrate | project-scoped, audience-bound tokens; agents cannot share, manage members or change policy (no scope for it); ops attributed and undoable; exports by agents audited | an agent can do anything its user can within the scope |
| token passthrough via MCP | a client hands us a token for another service | MCP MUST NOT accept or transit other tokens; audience check (A§3) | — |
| ex-employee | removed from the org | removal kills sessions and tokens at once (opaque tokens, §0 #5); SCIM deactivation in phase C | local copies already on their disk (§3) |
| content-hash probing | guessing a blob hash | per-request authorization of blob URLs (§3) | — |
| audit tampering | an insider deletes rows | hash chain + daily write-once export (§6) | the last day before export, detectable but not preventable |
| account takeover by email domain | an IdP asserting someone's email | accounts keyed by (iss, sub); linking only while signed in (§1) | — |

## 8. Cost to the speed budget

| path | added work | measured or estimated | budget |
|---|---|---|---|
| LOCAL edit (B5) | none | 0 instructions (no code on the path) | 0 — enforced by the gate |
| SERVED edit → commit (B2) | the role check inside the append | measured on a quiet machine (DESIGN §17.1, `phase2_pg.py`): the fenced, role-checked append 0.40 / 0.53 ms p50/p99 vs the bare insert 0.25 / 0.52 ms, i.e. +0.15 ms p50 for the fence's update and the role check together | **over the < 0.1 ms target** I set: still well inside B2 (the network is ≥ 1 ms), so it stands; if it matters, split the role check into a per-connection cache invalidated by NOTIFY and keep only the fence in the append |
| SERVED edit → collaborator (B2) | none beyond the commit | — | — |
| open a document | session or token lookup: one SHA-256 + one index read; effective-role query | µs (SHA-256) + an index lookup, the same order as the retry lookup measured at 0.03 ms (smoke) | once per connection |
| live connection | revalidation every 30 min; NOTIFY on grant changes | negligible | — |
| login | Argon2id 17.8 ms p50 (measured here, loaded CPU); passkey: one signature verify | not on any edit path | — |
| blob download | authorization + signed URL | one query per download | B3 adds < 1 ms to a download |

No per-op crypto, no extra round trip, nothing on the client's edit path. The one per-op cost
is the fence and role check in the append: +0.15 ms p50, measured (§17.1).

## 9. Migration from today

1. **LOCAL projects**: nothing changes, nothing to migrate.
2. **Phase-1 SERVED projects** (forge-server with `--user NAME:TOKEN` dev tokens, a
   `[remote.NAME] token = "…"` in the user's `remotes.toml`):
   - each dev user name maps to an account through an invite the server operator sends. On
     acceptance, `op_authors(old_name → account id)` is recorded, so history keeps its names and
     gains real identities;
   - the project becomes a personal project of the operator (or an org's), with each mapped
     user as editor;
   - the desktop replaces `token` with an OAuth sign-in on first connect. `remotes.toml` keeps
     the URL and the `projects` binding; the token moves to the keychain and the file field is
     ignored, then removed.
3. **forge-history versions** stay as they are. Trailers already name agents.

## 10. Phases

Each phase passes the speed gate with 0 regressions on local lines. Sizes as in DESIGN §11:
S ≤ 1 week, M 2–4 weeks, L 1–2 months, for one agent with review.

| phase | contents | size | exit |
|---|---|---|---|
| **A** (with DESIGN §17's phase 2) | accounts and identities; passkeys + passwords + Google/Microsoft OIDC; browser sessions; the authorization server for the desktop (code + PKCE + loopback, refresh rotation, keychain); personal projects; project roles; invites; view links; the role check in the append; blob authorization; the audit table + daily export; dev-token migration | **L** | two people share a project, one demoted mid-edit (op refused, connection downgraded), one revoked (connection closed); audit chain verified against the export |
| **B** | orgs, teams, base roles, policies, guests, verified domains; restricted projects; the hosted MCP resource server (RFC 9728 metadata, audience-bound scoped tokens, CIMD) | **M** | an agent edits through MCP with a project-scoped token, attributed; an org admin sees its audit; a guest cannot see the base-role projects |
| **C** | enterprise SSO (SAML 2.0 + OIDC), SCIM 2.0 deprovisioning, required-SSO for verified domains, audit streaming, retention settings; decide own-SP vs an IdP (§4) | **M–L** | SCIM deactivation kills every session within a minute; SSO-required members cannot use passwords |
| **D** (product decision) | release and approval workflows on named versions (Onshape's Pending/Released/Rejected/Obsolete, A§5) | **M** | — |

## 11. UNKNOWNs and how to find out

| item | how |
|---|---|
| passkeys through the system browser on Windows and Linux desktops (sourced only for macOS, A§8) | a spike on each OS with the RFC 8252 flow |
| absolute session timeout 12 h vs OWASP's 4–8 h example | beta: count passkey re-auth prompts per day |
| own SAML SP vs a self-hosted IdP | a one-week spike at phase C start |
| managed identity service prices | fetch vendors' price pages at phase C |
| the cost of encrypting served documents' local cache | only if customers ask for "no local copies" |
| the role check's cost in the append | measured: +0.15 ms p50 with the fence (§8); split it out only if B2 ever needs the time |
