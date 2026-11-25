# Auth prior art: users, orgs and authentication for a hosted multi-user CAD service

All sources fetched 2026-11-25 (web only; no sign-ups or logins; generic search terms only).
Quotes are as returned by the fetch tool. Some are short excerpts or close paraphrases of the page text; check the URL before quoting in a final document.
"UNVERIFIED" means I could not confirm the claim on a primary page during this session.

---

## 1. OAuth/OIDC for native and CLI apps

### RFC 8252, OAuth 2.0 for Native Apps (BCP 212) — https://www.rfc-editor.org/rfc/rfc8252

| Requirement | Quote (section) |
|---|---|
| External user-agent only | "native apps MUST use an external user-agent to perform OAuth authorization requests." (§5) |
| No embedded webviews | "native apps MUST NOT use embedded user-agents to perform authorization requests." (§8.12) |
| Why | The host app can "record every keystroke entered in the login form to capture usernames and passwords." (§8.12) |
| AS may block webviews | ASes "MAY take steps to detect and block authorization requests in embedded user-agents." (§8.12) |
| Loopback redirect | "Loopback redirect URIs use the "http" scheme and are constructed with the loopback IP literal" (§7.3) |
| Any port | "The authorization server MUST allow any port to be specified at the time of the request for loopback IP redirect URIs" (§7.3) |
| Avoid `localhost` | "the use of localhost is NOT RECOMMENDED" (§8.3), to avoid listening on interfaces other than loopback |
| Private-use scheme | "apps MUST use a URI scheme based on a domain name under their control" (§7.1) |
| PKCE | "Section 6 requires that both clients and servers use PKCE for public native app clients." (§8.1) |
| No client auth | "it is NOT RECOMMENDED for authorization servers to require client authentication of public native apps" (§8.5) |
| Embedded secrets are not secret | Secrets "statically included as part of an app distributed to multiple users should not be treated as confidential secrets" (§8.5) |

### RFC 7636, PKCE — https://www.rfc-editor.org/rfc/rfc7636

| Item | Quote (section) |
|---|---|
| Verifier length | "with a minimum length of 43 characters and a maximum length of 128 characters." (§4.1) |
| Entropy | "The client SHOULD create a "code_verifier" with a minimum of 256 bits of entropy." (§7.1) |
| S256 is mandatory to implement | "If the client is capable of using "S256", it MUST use "S256", as "S256" is Mandatory To Implement (MTI) on the server." (§4.2) |
| `plain` discouraged | ""plain" SHOULD NOT be used and exists only for compatibility with deployed implementations" (§7.2) |
| Server check | "BASE64URL-ENCODE(SHA256(ASCII(code_verifier))) == code_challenge"; a mismatch returns "invalid_grant" (§4.6) |

### RFC 8628, Device Authorization Grant — https://www.rfc-editor.org/rfc/rfc8628

| Item | Quote (section) |
|---|---|
| Scope | For "devices that have limited input capabilities or lack a suitable browser" (§1) |
| Not a replacement | It "is not intended to replace browser-based OAuth in native apps on capable devices." (§1) |
| Polling default | "If no value is provided, clients MUST use 5 as the default." (§3.2) |
| slow_down | "the interval MUST be increased by 5 seconds for this and all subsequent requests." (§3.5) |
| Brute force | "it is recommended that the server rate-limit user code attempts." (§5.1) |
| Remote phishing | "It is possible for the device flow to be initiated on a device in an attacker's possession." (§5.4) |
| Lifetime | `expires_in`: "The lifetime in seconds of the "device_code" and "user_code"." (§3.2) |

### RFC 9700, OAuth 2.0 Security BCP (BCP 240, January 2025) — https://www.rfc-editor.org/rfc/rfc9700

| Item | Quote (section) |
|---|---|
| Implicit grant | "clients SHOULD NOT use the implicit grant (response type `token`)" (§2.1.2) |
| Password grant | "The resource owner password credentials grant [RFC6749] MUST NOT be used." (§2.4) |
| PKCE | "Public clients MUST use PKCE"; for confidential clients "the use of PKCE [RFC7636] is RECOMMENDED" (§2.1.1) |
| Redirect matching | "authorization servers MUST utilize exact string matching" (§2.1) |
| Public-client refresh tokens | "Refresh tokens for public clients MUST be sender-constrained or use refresh token rotation" (§2.2.2) |
| Audience | "access tokens SHOULD be audience-restricted to a specific resource server" (§2.3) |

### OAuth 2.1 status — https://datatracker.ietf.org/doc/draft-ietf-oauth-v2-1/

- Still an Internet-Draft, not an RFC. The latest draft is **draft-ietf-oauth-v2-1-17**, dated 31 Oct 2026, and it expires 4 May 2027.
- The draft says features such as the Implicit and Resource Owner Password Credentials grants are "not specified in OAuth 2.1", and that features "such as PKCE, are required in OAuth 2.1."
- It requires "strict string matching of redirect URIs". The only exception is the port on loopback redirect URIs.
- Note: MCP's authorization spec (2026-07-28) still cites **draft-ietf-oauth-v2-1-13** (see §3).

---

## 2. Enterprise SSO

### OpenID Connect Core 1.0 (incorporating errata set 2, Final, 15 Dec 2023) — https://openid.net/specs/openid-connect-core-1_0.html

| Item | Quote (section) |
|---|---|
| Issuer | The Issuer Identifier "MUST exactly match the value of the iss (issuer) Claim." (§3.1.3.7 step 2) |
| Audience | "The Client MUST validate that the aud (audience) Claim contains its client_id value registered at the Issuer" (§3.1.3.7 step 3) |
| Expiry | "The current time MUST be before the time represented by the exp Claim." (§3.1.3.7 step 9) |
| Nonce | "a nonce Claim MUST be present and its value checked" when one was sent (§3.1.3.7 step 11) |
| Signature | Clients MUST validate the signature per JWS. For tokens received directly from the token endpoint, TLS server validation "MAY be used to validate the issuer in place of checking the token signature." (§3.1.3.7 step 6) |
| sub | "A locally unique and never reassigned identifier within the Issuer for the End-User" (§2) |
| Stable key = iss+sub | sub and iss "used together, are the only Claims that an RP can rely upon as a stable identifier for the End-User." (§5.7) |
| Email is not a key | email, phone_number, preferred_username and name "MUST NOT be used as unique identifiers" (§5.7) |

### SAML 2.0 today
- The OASIS overview says SAML "defines a framework for exchanging security information between online business partners." The page is marked Committee Draft 02, dated 25 March 2008. https://docs.oasis-open.org/security/saml/Post2.0/sstc-saml-tech-overview-2.0.html
- **What vendors support today.** These are observations from vendor docs:
  - Figma's authentication article lists only Google, email/password and **SAML SSO**, and does not mention OIDC. https://help.figma.com/hc/en-us/articles/360052497994-Choose-Organization-authentication-method
  - GitHub uses SAML for organization and enterprise SSO.
  - For Enterprise Managed Users, GitHub says "OpenID Connect (OIDC) and Conditional Access Policy (CAP) support for Enterprise Managed Users is only available for Microsoft Entra ID". Its docs describe migrating "from SAML to OIDC". https://docs.github.com/en/enterprise-cloud@latest/admin/managing-iam/configuring-authentication-for-enterprise-managed-users/about-support-for-your-idps-conditional-access-policy
  - Slack's SAML page lists "IDP Initiated Flow, SP-initiated flow, Just In Time provisioning, and automatic provisioning through our SCIM API". This came from a search snippet; the page body was not fetched (UNVERIFIED). https://slack.com/help/articles/205168057-custom-saml-single-sign-on
- **Inference, not a sourced claim:** an enterprise-facing SaaS still has to accept SAML. OIDC is the newer option and is IdP-specific where offered.

### SCIM 2.0 — RFC 7643 (schema) https://www.rfc-editor.org/rfc/rfc7643 and RFC 7644 (protocol) https://www.rfc-editor.org/rfc/rfc7644

| Item | Quote |
|---|---|
| Purpose | "an application-level HTTP-based protocol for provisioning and managing identity data on the web" (7644 §1) |
| Endpoints | `/Users` and `/Groups`: "Retrieve, add, modify Users/Groups" with GET, POST, PUT, PATCH, DELETE (7644 §3.2 table) |
| PATCH | "an OPTIONAL server function that enables clients to update one or more attributes of a SCIM resource." (7644 §3.5.2) |
| DELETE | "Service providers MUST return a 404 (Not Found) error code for all operations associated with the previously deleted resource." (7644 §3.6) |
| Bulk | An optional feature that "enables clients to send a potentially large collection" of operations (7644 §3.7) |
| `active` (soft deprovision) | "A Boolean value indicating the user's administrative status." (7643 §4.1.1) |
| `externalId` | "A String that is an identifier for the resource as defined by the provisioning client." (7643 §3.1) |

### Verified domains: how SaaS vendors tie an org to email domains

| Vendor | Mechanism | Quote / URL |
|---|---|---|
| Figma | Domain capture plus a DNS TXT record | "Domain capture lets you automatically add people with a company email address"; verified by "adding a unique code as a TXT record". With SSO enforced on unverified domains, "members will be prompted to verify their email address on each new device or browser". https://help.figma.com/hc/en-us/articles/360045953273 |
| Atlassian | Verify the domain (HTTPS file or DNS TXT), then claim accounts | "You can verify your company's domain to prove that you own all user accounts with that domain."; "You can't verify ownership of a public domain, such as `gmail.com`."; "When you claim accounts to manage, we let these users know that your organization manages their account." https://support.atlassian.com/user-management/docs/verify-a-domain-to-manage-accounts/ |
| Atlassian | One org per account | "You can only manage an Atlassian account in one organization." Search snippet only, from the Atlassian KB (UNVERIFIED). https://support.atlassian.com/atlassian-cloud/kb/cant-claim-accounts-from-verified-domain/ |
| GitHub (org) | DNS TXT | A verified domain gives a "Verified" badge and supports "restricting email notifications to only verified email domains". Verification works by creating "a DNS TXT record with your domain hosting service". https://docs.github.com/en/organizations/managing-organization-settings/verifying-or-approving-a-domain-for-your-organization |
| GitHub (enterprise) | Verified domains apply to all orgs | Enterprise verified domains apply to every org the enterprise owns. Search snippet of GHES docs only (UNVERIFIED for GHEC). https://docs.github.com/en/enterprise-server@3.17/admin/configuring-settings/configuring-user-applications-for-your-enterprise/verifying-or-approving-a-domain-for-your-enterprise |

### Plan tiers for SSO and SCIM

| Vendor | SSO tier | SCIM tier | Source |
|---|---|---|---|
| GitHub | "To use SAML single sign-on, your organization must use GitHub Enterprise Cloud." | Org-level SCIM requires SAML SSO, so it is also GHEC. Enterprise-level SCIM is "only available with Enterprise Managed Users" (search snippet). | https://docs.github.com/en/enterprise-cloud@latest/organizations/managing-saml-single-sign-on-for-your-organization/about-identity-and-access-management-with-saml-single-sign-on |
| Figma | "Education (K-12), Organization, Government, and Enterprise plans" | Pre-provisioning via SCIM is on the same plans. Setting seat types via SCIM is "Enterprise plan" only. | https://help.figma.com/hc/en-us/articles/360040532333-Guide-to-Figma-plans ; https://help.figma.com/hc/en-us/articles/360052497994-Choose-Organization-authentication-method |
| Slack | "Available on the Business+ and Enterprise plans" (also Free/Pro with a connected Salesforce org) | SCIM is also "Available on the Business+ and Enterprise plans" | https://slack.com/help/articles/203772216-SAML-single-sign-on ; https://slack.com/help/articles/212572638-Manage-members-with-SCIM-provisioning |
| Atlassian | Requires "Atlassian Guard Standard", a paid org-wide add-on. Domain verification itself is on "All plans". | SCIM requires "a subscription to Atlassian Guard Standard" | https://support.atlassian.com/security-and-access-policies/docs/configure-saml-single-sign-on-with-an-identity-provider/ |

---

## 3. MCP authorization (current protocol version **2026-07-28**)

The current version is confirmed here: "The **current** protocol version is **2026-07-28**." https://modelcontextprotocol.io/specification/versioning
Spec pages:
- https://modelcontextprotocol.io/specification/latest/basic/authorization
- https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization/authorization-server-discovery.md
- https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization/client-registration.md

| Topic | Requirement (quote) |
|---|---|
| Scope of the spec | "Authorization is **OPTIONAL**". HTTP transports "**SHOULD** conform". STDIO "**SHOULD NOT** follow this specification, and instead retrieve credentials from the environment." |
| Role | "A protected *MCP server* acts as an OAuth 2.1 resource server" |
| Protected resource metadata (RFC 9728) | "MCP servers **MUST** implement OAuth 2.0 Protected Resource Metadata"; the document "**MUST** include the `authorization_servers` field containing at least one authorization server." |
| Discovery | Servers MUST implement one of: `resource_metadata` in `WWW-Authenticate` on 401, or the well-known URI (path-specific `/.well-known/oauth-protected-resource/<path>` or root). Clients MUST support both. |
| AS metadata | ASes "**MUST** provide at least one" of RFC 8414 or OIDC Discovery. Clients MUST try both. The `issuer` "**MUST** be identical" to the one used to build the URL. |
| Resource indicators (RFC 8707) | Clients "**MUST** implement Resource Indicators". `resource` "**MUST** be included in both authorization requests and token requests". Clients MUST send it "regardless of whether authorization servers support it." |
| Audience | "MCP servers **MUST** validate that access tokens were issued specifically for them as the intended audience" |
| No passthrough | "MCP servers **MUST NOT** accept or transit any other tokens." |
| Token transport | `Authorization: Bearer` on every request. Tokens "**MUST NOT** be included in the URI query string". |
| Issuer mix-up (RFC 9207) | Clients MUST record the expected issuer and validate `iss` in the authorization response. ASes "**SHOULD** include the `iss` parameter". |
| Client registration | Priority order: pre-registered, then **Client ID Metadata Documents** ("**SHOULD** support"), then DCR as a fallback, then asking the user. |
| DCR (RFC 7591) | "**MAY** support". **"Deprecated: Dynamic Client Registration is deprecated as of protocol version `2026-07-28`"**, with at least 12 months before it can be removed. DCR clients "**MUST** specify an appropriate `application_type`" (native vs web). |
| Scopes / step-up | 403 with `error="insufficient_scope"`, `scope=…`, `resource_metadata`. Servers "**SHOULD** include all scopes required for the current operation in a single challenge". Clients compute the union with previously granted scopes. |
| Refresh | Clients "**MUST NOT** assume refresh tokens will be issued". Servers "**SHOULD NOT** include `offline_access`" in scope challenges. |
| Error codes | 401 = missing or invalid token; 403 = insufficient scope or permissions; 400 = malformed request. |

Supporting RFCs:
- RFC 9728 (April 2025) https://www.rfc-editor.org/rfc/rfc9728 :
  - default path "/.well-known/oauth-protected-resource";
  - `resource` is "REQUIRED";
  - `authorization_servers` is a "JSON array containing a list of OAuth authorization server issuer identifiers";
  - `resource_metadata` in `WWW-Authenticate` is "The URL of the protected resource metadata."
- RFC 8707 https://www.rfc-editor.org/rfc/rfc8707 :
  - `resource` "MUST be an absolute URI" and "MUST NOT include a fragment component";
  - the AS "SHOULD audience-restrict issued access tokens to the resource(s) indicated";
  - error `invalid_target`.

---

## 4. Token handling

### JWT vs opaque tokens, and revocation

| Source | Quote |
|---|---|
| RFC 7519 (JWT) https://www.rfc-editor.org/rfc/rfc7519 | "JSON Web Token (JWT) is a compact, URL-safe means of representing claims to be transferred between two parties."<br>`exp`: "on or after which the JWT MUST NOT be accepted for processing" (§4.1.4)<br>`jti` "can be used to prevent the JWT from being replayed" (§4.1.7)<br>**Defines no revocation mechanism.** |
| RFC 9068 (JWT access-token profile) https://www.rfc-editor.org/rfc/rfc9068 | "The JWT access token MUST be rejected if "aud" does not contain a resource indicator of the current resource server" (§4)<br>Embedded claims let RSes avoid "further round trips to introspection" (§2.2.2) |
| RFC 7662 (introspection) https://www.rfc-editor.org/rfc/rfc7662 | `active`: "Boolean indicator of whether or not the presented token is currently active." (§2.2)<br>"The response MAY be cached by the protected resource to improve performance." (§2.2)<br>Caching "creates a window during which a revoked token could be used at the protected resource." (§4) |
| RFC 7009 (revocation) https://www.rfc-editor.org/rfc/rfc7009 | Revoking a refresh token: the AS "SHOULD also invalidate all access tokens based on the same authorization grant" (§2.1)<br>Self-contained tokens need "some (currently non-standardized) backend interaction between the authorization server and the resource server" (§3) |
| OWASP Top 10 A01 https://top10.owasp.org/2021/A01_2021-Broken_Access_Control/index.html | Stateless JWTs should be short-lived to limit the attacker's window. Stateful session IDs should be invalidated after logout. (paraphrase) |

Tradeoff in one line (synthesis): a JWT can be validated locally but cannot be revoked before `exp` without extra machinery. An opaque token plus introspection can be revoked at once, at the cost of a round trip or a cache window.

### Refresh-token rotation (RFC 9700 §4.14.2) — https://www.rfc-editor.org/rfc/rfc9700
- "the authorization server issues a new refresh token with every access token refresh response."
- "The previous refresh token is invalidated, but information about the relationship is retained by the authorization server." If an invalidated token is replayed, "The authorization server cannot determine which party submitted the invalid refresh token," so it revokes the active refresh token.
- "Refresh tokens SHOULD expire if the client has been inactive for some time,"
- ASes "MAY revoke refresh tokens automatically in case of a security event", for example a password change or logout.
- The OWASP OAuth2 cheat sheet agrees: "Combining PoP-constrained refresh tokens with rotation provides defense-in-depth." https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html

### OWASP Session Management Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html

| Item | Quote |
|---|---|
| Entropy | "Session identifiers must have at least 64 bits of entropy" |
| Secure | "only send the cookie through an encrypted HTTPS" channel |
| HttpOnly | "instructs web browsers not to allow scripts" to access the cookie |
| SameSite | "Session cookies must explicitly set `SameSite=Strict` (preferred) or `SameSite=Lax`." |
| Domain/Path | "use a narrow or restricted scope for these two attributes." |
| Prefix | `__Host-` is "Recommended for session IDs." |
| Idle timeout | "2-5 minutes for high-value applications and 15-30 minutes for low risk applications." |
| Absolute timeout | "between 4 and 8 hours" (for a full-day office workflow) |
| Renewal | "The session ID must be renewed or regenerated by the web application after any privilege level change" |
| Server-side | "Session timeout management and expiration must be enforced server-side." |
| No web storage | "Do not store authentication tokens, session IDs, JWTs, refresh tokens" in localStorage or sessionStorage |

### Desktop token storage (OS keychain)

| Platform | Quote / URL |
|---|---|
| Apple Keychain | "Many apps need to handle passwords and other short but sensitive bits of data, such as keys and login tokens."; items are "encrypted using two different AES-256-GCM keys". https://support.apple.com/guide/security/keychain-data-protection-secb0694df1a/web. Note: developer.apple.com Keychain Services pages render client-side, and the fetch returned only the title. |
| Windows Credential Locker | Lets apps "securely store and retrieve user credentials, and roam them between devices with the user's Microsoft account". Callable from WinUI, WPF and WinForms. Limit "up to 20 credentials per app". "Never store credentials in plain-text using app data or roaming settings." https://learn.microsoft.com/en-us/windows/apps/develop/security/credential-locker |
| Windows DPAPI | "only a user with the same logon credential as the user who encrypted the data can decrypt the data"; usually only "on the same computer". The PromptStruct flow is deprecated and will be removed in **April 2027**. https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata |
| Linux Secret Service (GNOME Keyring/KWallet) | The spec page says only "The Secret Service manages all the sessions and collections." It has no purpose statement. http://specifications.freedesktop.org/secret-service/latest/ |
| RFC 8252 | Does not say where native apps should store tokens (UNVERIFIED for the full text; the fetched part had no storage guidance). |

---

## 5. Org, team and role models of comparable products

### Onshape (CAD)

| Concept | Quote / URL |
|---|---|
| Global permissions | "Global permissions control user access to Enterprise-level operations and information." There are 25 of them, including Enterprise administrator; Manage users, teams, and aliases; Invite guest users; Create projects; Create Releases; Approve Releases; Enable link sharing; Transfer documents out of Enterprise; Export files; Make personal copies. https://cad.onshape.com/help/Content/Plans/enterprise_settings_global_permissions.htm |
| Teams | "You can create teams in order to group users together for the purpose of sharing documents more efficiently." Members only need to belong to the same enterprise. https://cad.onshape.com/help/Content/teams-enterprise.htm |
| Projects | "Permission schemes are a named table of Role-Permission set pairs."; "Project roles have no meaning or effect until associated with permissions and users in a Permission scheme." https://cad.onshape.com/help/Content/Plans/project_roles_and_permission_schemes.htm |
| Project inheritance | "the permissions you apply to the Projects affect all documents inside the Project as well." Moving a document between projects "requires full permission on both the source Project and the target project", and the moved item takes the target project's permissions. https://cad.onshape.com/help/Content/EnterpriseHelp/Content/projects.htm |
| Folder inheritance | "all Share permissions on a folder are inherited by the documents, publications, and folders within that folder"; "permissions are additive: these additional permissions are added to the folder permissions". The docs advise to "keep minimal permissions on the root folder". https://cad.onshape.com/help/Content/Document/folders.htm |
| Share levels | Can edit = "edit, copy, link, export, share, and comment"; Can view = "view, copy, link, export, and comment". Separate checkboxes: Copy, Link document (needs Export), Export, Share (reshare), Comment, Delete. Link share: "Anyone with the link will have access", read-only, "viewable without an Onshape account". https://cad.onshape.com/help/Content/Collaboration/share_documents.htm |
| Release workflow | "Release management is a set of automated workflows"; states are Pending, Released, Rejected, Obsolete. "An approver must have permission to edit the document in order to approve or reject a Release candidate." "Enterprise users can additionally create custom release and obsoletion workflows". https://cad.onshape.com/help/Content/Release/release_management.htm |
| Overrides | **Grant more:** yes, document shares are additive. **Grant less than the folder or project:** not documented, and implied impossible (UNVERIFIED). |
| Plan tiers | The pages I fetched don't say which plan includes projects or release management (UNVERIFIED). |
| Doc inconsistencies | Search snippets show Onshape pages disagree on whether changing a project needs "Share" or "Reshare" permission. One older page says guests cannot hold global permissions; a newer one says they can. Both are UNVERIFIED. |

### Figma

| Concept | Quote / URL |
|---|---|
| **Projects renamed to folders (Aug 2026)** | "Projects are now called folders."; "On paid plans, you can now create folders inside other folders, up to 10 levels deep." Rolled out from August 3, 2026. The old Team Access settings became "Inherited" and "Limited", and "View only" was removed. https://help.figma.com/hc/en-us/articles/41753150926103-Updates-to-Figma-s-file-management |
| Roles | Owner, Can edit ("Full file editing access"), Can view ("View and comment on the file, but not edit it"). https://help.figma.com/hc/en-us/articles/35361119554711-File-and-folder-permissions |
| Inheritance | "Files always inherit user roles from the folder they're in, and you can't turn that off on an individual file." A file can grant **more** access than the folder role, not less. On Org and Enterprise plans you can "limit any folder (top-level or nested) to specific people". (same URL) |
| Audience | "Audience access controls whether your wider plan or workspace can reach a file or folder." (same URL) |
| Removing access | Removing someone from a file "only removes their explicit access"; they "may still access the file via inherited permissions" (search snippet). https://help.figma.com/hc/en-us/articles/360040530793-Remove-or-adjust-access |
| Admin roles | Organization admin ("highest level of administrative access"; Org and Enterprise plans). Workspace admin ("a level of access below an organization admin"; Enterprise only). Billing group admin (Enterprise only). Team admin ("manage members, permissions, libraries, and settings for a specific team"). https://help.figma.com/hc/en-us/articles/4420557724439-Admins-in-Figma |
| Seats | Full (edit Design, Dev Mode, Slides, FigJam, Make); Dev; Collab; View ("View and comment in files they have access to"). https://help.figma.com/hc/en-us/articles/360039960434-Roles-and-account-types-in-organizations |

### GitHub

| Concept | Quote / URL |
|---|---|
| Org roles | Owners ("complete administrative access"), Members, Moderators, Billing managers, Security managers, Outside collaborators, and custom org roles (GHEC). https://docs.github.com/en/organizations/managing-peoples-access-to-your-organization-with-roles/roles-in-an-organization |
| Repo roles | Read, Triage, Moderate (as listed on the page), Write, Maintain, Admin. "If your organization uses GitHub Enterprise Cloud, you can create custom repository roles." https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/managing-repository-roles/repository-roles-for-an-organization |
| Base permissions | "You can set base permissions that apply to all members of an organization."; "the higher level of access overrides the base permission". https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/managing-repository-roles/setting-base-permissions-for-an-organization |
| Teams | "Child teams inherit the parent's access permissions"; "Teams can be visible or secret." https://docs.github.com/en/organizations/organizing-members-into-teams/about-teams |
| Conflicts | "If a person has been given conflicting access, you'll see a warning on the repository access page" ("Mixed roles"). https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/managing-teams-and-people-with-access-to-your-repository |
| Overrides | Additive, highest grant wins: a repo can grant **more** than base or team access. A repo cannot grant a member **less** than base permission; this is implied by "overrides" but not stated (UNVERIFIED). The REST `role_name` field is "the highest role… after considering all sources of grants" (search snippet; UNVERIFIED). |

### Google Drive / Docs link sharing — https://support.google.com/drive/answer/7166529?hl=en

| Concept | Quote |
|---|---|
| General access | Restricted ("Only people who have been given access can open the file"), Anyone with the link (no sign-in), or Your organization |
| Roles | Viewer, Commenter, Editor ("open, edit, delete, or move any files within the folder") |
| Inheritance | "When you share a folder, all files and subfolders inside it inherit those same sharing permissions." |
| Less than folder | "You cannot give someone less access to a specific file than they have to its parent folder." |
| Folder wins | "the folder's permissions override any settings you choose for the file." Separately, "Limit access" restricts a folder to people explicitly added. |
| Shared drives | Manager, Content manager, Contributor, Commenter, Viewer. "You can't make the access to folders more restrictive than the shared drive itself." "Limited-access folders are an exception to these rules." https://support.google.com/a/users/answer/12380484?hl=en |

### Summary: can a file override its folder or project?

| Product | Grant more on a file | Grant less on a file | Restrict a subtree |
|---|---|---|---|
| Onshape | Yes (additive) | Not documented (implied no) | Not documented |
| Figma | Yes | No | Yes, a "Limited" folder (Org/Enterprise only) |
| GitHub | Yes (highest wins) | No (implied) | n/a (repo is the unit) |
| Google Drive | Yes | No | Yes, a limited-access folder |

---

## 6. Audit logs

| Product | What is recorded | Retention | URL |
|---|---|---|---|
| GitHub Enterprise Cloud | "The user (actor) who performed the action," "The action that was performed," "The date and time"; IP optional ("You can display the IP address"). Search, export, and streaming ("stream audit and Git events data… to an external data management system"). | "within the last 180 days"; "The audit log retains Git events for seven days." | https://docs.github.com/en/enterprise-cloud@latest/admin/monitoring-activity-in-your-enterprise/reviewing-audit-logs-for-your-enterprise/about-the-audit-log-for-your-enterprise |
| Figma activity logs | Sign-ins and sign-outs (none for guests); "Viewed a file," "Exported a file," "File link access changed," "File membership changed"; "Exported the members list," "Org membership changed"; IP address per event. CSV by email. On Org, Enterprise, Gov and Edu plans; not tracked for Starter or Professional. Logs start at upgrade and are not retroactive. | **Not stated** on the help page. The API's `start_time` "defaults to one year ago if unspecified"; a 1-year retention figure appears only in a community forum post (UNVERIFIED). | https://help.figma.com/hc/en-us/articles/360040449533-View-and-export-activity-logs ; https://developers.figma.com/docs/rest-api/activity-logs-endpoints |
| Figma Activity Logs API | "only available to organizations on the Enterprise plan"; scope `org:activity_log_read` | — | https://developers.figma.com/docs/rest-api/activity-logs/ |
| Figma developer logs | "API calls made to the REST API or MCP server" (Enterprise + Governance+) | "retained for the last 30 days" | https://developers.figma.com/docs/rest-api/developer-logs/ |

### OWASP Logging Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html

- **Always log:**
  - "Authentication successes and failures"
  - "Authorization (access control) failures"
  - "Session management failures"
  - "Input validation failures"
  - "changes to privileges"
  - all actions by administrators
  - "Data import and export"
- **Attributes:** the "when, where, who and what": timestamps, app/service IDs, source address, user identity, event type, severity.
- **Exclude:** passwords, access tokens and encryption keys. If you need to track a session, record a hash of the session ID.
- **Integrity:**
  - "Build in tamper detection so you know if a record has been modified or deleted."
  - "Store or copy log data to read-only media as soon as possible."
  - Log access to the logs themselves.
- **Retention:** do not destroy logs before "the duration of the required data retention period". The period is driven by legal, regulatory and contract requirements.

---

## 7. Threat-model references

### OWASP ASVS
The current stable release is **5.0.0**, "Published 30 May 2025". https://asvs.owasp.org/

| ASVS 5.0 chapter (from repo filenames) | Relevance |
|---|---|
| V6 Authentication | login, MFA, credentials |
| V7 Session Management | sessions, timeouts |
| V8 Authorization | access control |
| V9 Self-contained Tokens | JWTs |
| V10 OAuth and OIDC | new in v5 |
| V4 API and Web Service | includes WebSocket topics (UNVERIFIED: not opened) |
| V16 Security Logging and Error Handling | audit logs |

Source: https://github.com/OWASP/ASVS/tree/master/5.0/en

For **ASVS 4.0.3**, the chapters V2 Authentication, V3 Session Management and V4 Access Control are UNVERIFIED (from memory; not fetched).

### OWASP Top 10
- **2021 A01 Broken Access Control.** https://top10.owasp.org/2021/A01_2021-Broken_Access_Control/index.html
  - "Moving up from the fifth position, 94% of applications were tested for some form of broken access control."
  - "Access control enforces policy such that users cannot act outside of their intended permissions."
  - Common weaknesses include IDOR ("providing its unique identifier"), "Elevation of privilege", "CORS misconfiguration", and JWT replay or tampering.
  - Prevention includes "Except for public resources, deny by default." and "Model access controls should enforce record ownership".
- **A 2025 edition exists.** top10.owasp.org now redirects to "OWASP Top 10:2025", where A01:2025 is still "Broken Access Control" and A07:2025 is "Authentication Failures". The page shows no final or RC label and no release date. https://top10.owasp.org/2025/

### CSWSH (cross-site WebSocket hijacking)
- **PortSwigger** says CSWSH "involves a cross-site request forgery (CSRF) vulnerability on a WebSocket handshake" and "arises when the WebSocket handshake request relies solely on HTTP cookies for session handling". https://portswigger.net/web-security/websockets/cross-site-websocket-hijacking
- **PortSwigger's securing guidance** (https://portswigger.net/web-security/websockets):
  - "Use the `wss://` protocol"
  - "Hard code the URL of the WebSockets endpoint"
  - "Protect the WebSocket handshake message against CSRF"
  - "Treat data received via the WebSocket as untrusted in both directions."
  - PortSwigger does not explicitly name Origin checks.
- **The OWASP WebSocket Security Cheat Sheet** gives the Origin defence. https://cheatsheetseries.owasp.org/cheatsheets/WebSocket_Security_Cheat_Sheet.html
  - "Validate the `Origin` header on every handshake"
  - "Always use an explicit allowlist of trusted origins" (no wildcards or substring matching)
  - Avoid tokens in URL query strings "because they can leak into access logs"
  - "Close WebSocket connections when sessions expire"
  - "Re-validate user sessions periodically (every 30 minutes is common)"
  - "Check authorization for each action."
  - The fetched text did not say that non-browser clients can forge Origin. That is general knowledge and is UNVERIFIED here.

---

## 8. WebAuthn / passkeys

| Item | Quote / URL |
|---|---|
| WebAuthn Level 3 status | **"W3C Recommendation, 25 August 2026."** https://www.w3.org/TR/webauthn-3/ |
| FIDO passkey definition | "A passkey is an authentication credential based on FIDO standards, that can be stored on your phone or computer…" Passkeys "can be securely synced across a user's devices, or bound to a particular device" (synced vs device-bound). https://fidoalliance.org/passkeys/ |
| Windows native API | Win32 WebAuthn platform APIs exist since Windows 10 1903. Windows 11 24H2 adds "plugin passkey managers". "As a relying party, a native application can also act as a WebAuthn client to make direct WebAuthn calls." https://learn.microsoft.com/en-us/windows/security/identity-protection/hello-for-business/webauthn-apis |
| Apple: OAuth in-app | Apple staff engineer: "if you're trying to authenticate your own app via a web sign in (such as OAuth) you'll want to use `ASWebAuthenticationSession`" (forum answer, Jan 2025). https://developer.apple.com/forums/thread/771848 |
| macOS system browser | "ASWebAuthenticationSession is the System WebView (SWV) on macOS for authentication flows". It opens the default browser, so passkeys work for "sign in flows on third party domains". Embedded WebViews can only use passkeys "for the linked web domain (RP ID)". https://passkeys.dev/docs/reference/macos/ |
| Known issue | A developer forum thread reports passkey autofill not triggering in ASWebAuthenticationSession after iOS 17.4. This is iOS, not desktop, and unresolved (search snippet; UNVERIFIED). https://developer.apple.com/forums/thread/763067 |
| Inference | A desktop app that follows RFC 8252 (system browser plus loopback redirect) gets passkeys for free, because the browser runs WebAuthn at the AS's origin. No source states this for Windows or Linux desktop specifically (UNVERIFIED as a sourced claim). |
