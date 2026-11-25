# Onshape REST import: scoping (not built)

What a direct "Import from Onshape" would take: ForgeCAD fetches a Part Studio or Assembly as
Parasolid over Onshape's REST API and hands the bytes to `forge_step::import`, which already reads
everything Onshape writes (v25 to v37 .x_t/.x_b, multi-part list roots, nested assemblies, names:
`kernel/forge-step/tests/xt.rs`). Nothing here was signed up for or called. Every fact comes from
Onshape's public developer docs (onshape-public.github.io/docs, read 2026-11). Items marked
**unconfirmed** need a real account to settle.

## Authentication

Two routes, both using the user's own account. ForgeCAD holds no Onshape credentials of its own.

- **API keys** (docs/auth/apikeys). The user creates an access/secret pair in their Onshape
  account settings; individual accounts get at most two active keys. Requests are signed with
  HMAC-SHA256. The signed string is the method, `On-Nonce`, `Date`, `Content-Type`, path and
  query, one per line and lowercased. The headers are `Authorization: On <access>:HmacSHA256:<sig>`,
  `Date` (within 5 minutes) and `On-Nonce` (at least 16 alphanumerics). Basic auth also works but
  is "for local testing only". Onshape says keys are for personal or internal use. Keys work only
  on the stack where they were made (an enterprise's own domain or cad.onshape.com). Which plans
  may create keys: **unconfirmed**.
- **OAuth2** (docs/auth/oauth). An app is registered under Developer Settings. Flow:
  authorization code at `https://oauth.onshape.com/oauth/authorize`, then
  `POST https://oauth.onshape.com/oauth/token`. Access tokens last 60 minutes; refresh tokens last
  for the life of the grant and rotate on each refresh. A desktop app can register
  `http://localhost:<port>` as its redirect. Each user grants the app in their own account.
  Onshape requires OAuth2 for App Store distribution. Scope names (a read scope is all an import
  needs): **unconfirmed**.

Recommendation: ship API keys first (the user pastes their pair into a settings field, stored in
the OS keychain; no app registration or developer agreement needed). Add OAuth2 only if ForgeCAD
is listed in the App Store. App Store listing also takes a signed developer agreement and
Onshape's testing (docs/app-store/checklist).

## Endpoints

Base: `https://cad.onshape.com/api/v<N>/…` (or `<company>.onshape.com`). Paths take
`/d/{did}/{w|v|m}/{wvmid}/e/{eid}`. A pasted Onshape URL
(`cad.onshape.com/documents/{did}/w/{wid}/e/{eid}`) gives all three IDs, so no document browser
is needed at first.

| Need | Call | Status |
|---|---|---|
| One part as Parasolid, synchronous | `GET /parts/d/{did}/w/{wid}/e/{eid}/partid/{pid}/parasolid?version=0`, `Accept: application/octet-stream` | documented |
| Part Studio as Parasolid, synchronous | `PartStudio/exportParasolid`, likely `GET /partstudios/d/…/e/{eid}/parasolid` | operation named, path **unconfirmed** |
| Assembly as Parasolid | `POST /assemblies/d/{did}/{wv}/{wvid}/e/{eid}/translations` with `{"formatName":"PARASOLID","parasolidMode":"BINARY","storeInDocument":false}` | documented |
| Part Studio, asynchronous | `POST /partstudios/d/…/e/{eid}/translations` with `formatName: PARASOLID` | generic endpoint documented, Parasolid body **unconfirmed** |
| Poll a translation | `GET /translations/{tid}` until `requestState` is `DONE` or `FAILED` (`failureReason`), with exponential backoff | documented |
| Fetch the result | `GET /documents/d/{did}/externaldata/{fid}` (from `resultExternalDataIds`) | documented |
| Part names, numbers | `GET /parts/d/{did}/w/{wid}/e/{eid}`: `name`, `partId`, `partNumber`, `defaultColorHash` | documented |
| Properties | `GET /metadata/d/{did}/w/{wid}/e/{eid}[/p/{pid}]` | documented |
| Appearances / colours | none found | **unconfirmed** |

Synchronous exports answer with a 307 redirect, and the redirected request needs its own
signature. With `storeInDocument: false`, the translation leaves nothing in the user's document.

Parameters still to confirm on a real account: the Parasolid `version`, text vs binary for Part
Studios, `includeExportIds`, configurations and flattening.

## Limits

- **Annual call quotas** (docs/auth/limits). Free, Standard and EDU Student: 2,500 calls per user
  per year. Professional: 5,000 per user. Enterprise: 10,000 per full user. Only 2xx and 3xx
  responses count. Calls from public App Store OAuth apps do not count. Once the quota is spent,
  calls get **402**.
- **Rate limits** are per endpoint and not published. Every response carries
  `X-Rate-Limit-Remaining`; a **429** carries `Retry-After` (seconds), which must be honoured.
- **Cost per import:** one export is about 1 call synchronous, or about 3 to 10 calls through a
  translation (start, polls, download), plus 1 for part names. On a free account that is a few
  hundred imports a year, so polls should back off (1 s, 2 s, 4 s and so on). ForgeCAD should show
  the remaining quota when Onshape reports it.

## Terms

Onshape's API terms say the API is for tools "that enhance productivity for you or other Onshape
users" and bar automated scraping of public documents. An import that runs on the user's
request, on their own documents and with their own credentials, fits that. Distributing an
OAuth2 app outside the App Store is not addressed: **unconfirmed**. The full API Terms of Use
were not read.

## Shape of the work (when approved)

1. `forge-onshape` (native only): parse a pasted URL, sign requests (HMAC-SHA256 with the SHA-2
   already in-tree, or a small new one), follow the 307 redirect, run the translation and poll
   loop with backoff, and handle 402/429/`Retry-After`. Credentials go in the OS keychain, never in
   project files.
2. UI: File > Import from Onshape… (URL field, key setup once). CLI: `forgecad import
   onshape:<url>`. MCP: the same, gated on a writable session.
3. Tests: a recorded-response fake server (no live calls in CI), signature test vectors taken
   from the docs' example, and the bytes going through the existing XT tests.

Estimate: about 3 to 4 days for API keys and a pasted URL. OAuth2 and a document browser are
extra, about 3 days, plus the App Store process.
