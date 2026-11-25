# Phase 2 prior art: cited facts

All fetched **2026-11-25** unless noted. Monthly = hourly x 730 h, rounded to cents, computed by me (not quoted by the vendor unless stated). Prices in USD unless marked EUR. "Fetched via price feed" means the JSON file that the vendor's public pricing page loads (URL given). No accounts or logins were used.

---

## 1. Compute prices (on-demand, Linux)

### AWS EC2, US East (N. Virginia)
Source: the JSON price feed behind https://aws.amazon.com/ec2/pricing/on-demand/, at
`https://b0.p.awsstatic.com/pricing/2.0/meteredUnitMaps/ec2/USD/current/ec2-ondemand-without-sec-sel/US%20East%20(N.%20Virginia)/Linux/index.json` (manifest `hawkFilePublicationDate: 2026-11-24T23:14:17Z`).
Spot: the feed behind https://aws.amazon.com/ec2/spot/pricing/, at `https://website.spot.ec2.aws.a2z.com/spot.js`. This is a live snapshot of us-east-1 spot prices. Spot prices float.

| Instance | vCPU / RAM | On-demand $/h | $/mo (730 h) | Spot $/h (snapshot) | Spot $/mo | Spot discount |
|---|---|---|---|---|---|---|
| m7i.large | 2 / 8 GiB | 0.1008 | 73.58 | 0.0449 | 32.78 | 55% |
| c7i.2xlarge | 8 / 16 GiB | 0.3570 | 260.61 | 0.163 | 118.99 | 54% |
| m7i.2xlarge | 8 / 32 GiB | 0.4032 | 294.34 | 0.1774 | 129.50 | 56% |
| (ref) m7g.large (Graviton) | 2 / 8 GiB | 0.0816 | 59.57 | – | – | – |
| (ref) c7g.2xlarge (Graviton) | 8 / 16 GiB | 0.2900 | 211.70 | – | – | – |

### GCP Compute Engine, Iowa (us-central1)
The pricing page's default region selector is `aria-selected="true" data-value="Iowa (us-central1)"`. I did not extract us-east1 or us-east4 rows, so treat any us-east figure as UNVERIFIED.
On-demand: https://cloud.google.com/products/compute/pricing/general-purpose. The C3 rows appear on this page too; it is the redirect target of /compute/vm-instance-pricing.
Spot: https://cloud.google.com/spot-vms/pricing (column "Current Spot pricing (USD)").
- Quote: "Spot VMs ... provide discounts of up to 91% off of the corresponding default price". The page also says Spot prices can change "up to once every day".

| Machine type | vCPU / RAM | On-demand $/h | $/mo | Spot $/h | Spot $/mo | Spot discount |
|---|---|---|---|---|---|---|
| e2-standard-2 | 2 / 8 GiB | 0.06701142 | 48.92 | 0.040212 | 29.35 | 40% |
| c3-highcpu-8 | 8 / 16 GiB | 0.340208 | 248.35 | 0.120544 | 88.00 | 65% |
| c3-standard-8 | 8 / 32 GiB | 0.403216 | 294.35 | – | – | – |
| n2-standard-8 | 8 / 32 GiB | 0.388472 | 283.58 | 0.233024 | 170.11 | 40% |
| (ref) n2-highcpu-8 | 8 / 8 GiB | 0.286784 | 209.35 | 0.172016 | 125.57 | 40% |

### Hetzner Cloud (EUR, excl. VAT)
Sources: https://www.hetzner.com/cloud/general-purpose/ (CCX, dedicated vCPU) and https://www.hetzner.com/cloud/regular-performance/ (CPX, shared vCPU). The pages show only EUR, although a USD toggle exists.
- Quote: "Your server's bill will never exceed its monthly price cap."
- Every plan's Create button read "This product is currently unavailable. Please check back later." at fetch time.
- Hourly prices are shown once per plan. Monthly caps are per location.
- The pages show **no spot or preemptible offering**.

| Plan | vCPU / RAM | €/h | Monthly cap HEL1 (FI) | NBG1 (DE) | ASH1 / HIL1 (US) | Included traffic |
|---|---|---|---|---|---|---|
| CCX13 (dedicated) | 2 / 8 GB | 0.0593 | €36.99 | €43.49 | €43.99 | 20 TB EU / 1 TB US |
| CCX33 (dedicated) | 8 / 32 GB | 0.1891 | €117.99 | €138.99 | €141.49 | 30 TB EU / 3 TB US |
| CPX32 (shared, EU) | 4 / 8 GB | 0.0489 | €30.49 | €35.99 | n/a | 20 TB |
| CPX42 (shared, EU) | 8 / 16 GB | 0.0953 | €59.49 | €69.99 | n/a | 20 TB |
| CPX41 (shared, US only) | 8 / 16 GB | 0.1939 | n/a | n/a | €120.99 | 4 TB |

- Extra traffic, quoted as the page prints it: "For each additional TB at EU-Central locations we charge € 1.00 per hour, excl. VAT". "per hour" looks like a page typo for per TB, so the unit is UNVERIFIED.

---

## 2. Managed Postgres

### AWS RDS for PostgreSQL, us-east-1
Instance prices come from the feed behind https://aws.amazon.com/rds/postgresql/pricing/, at `https://b0.p.awsstatic.com/pricing/2.0/meteredUnitMaps/rds/USD/current/rds-postgresql-ondemand.json` (published 2026-11-24T23:30:28Z). Storage prices come from `.../rds/USD/current/rds-storage.json`.

| Item | $/h | $/mo |
|---|---|---|
| db.m7g.large (2 vCPU / 8 GiB), Single-AZ | 0.168 | 122.64 |
| db.m7g.large, Multi-AZ (one standby) | 0.337 | 246.01 |
| (ref) db.m8g.large Single / Multi-AZ | 0.168 / 0.336 | 122.64 / 245.28 |
| (ref) db.t4g.medium Single / Multi-AZ | 0.065 / 0.129 | 47.45 / 94.17 |
| gp3 storage, Single-AZ | – | $0.115 per GB-month |
| gp3 storage, Multi-AZ | – | $0.23 per GB-month |
| gp3 provisioned IOPS above baseline, Single / Multi-AZ | – | $0.02 / $0.04 per IOPS-month |

**Failover times (documented):**
- Multi-AZ DB *instance* (one standby): "Failover times are typically 60–120 seconds. However, large transactions or a lengthy recovery process can increase failover time." Source: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/Concepts.MultiAZ.Failover.html. **This confirms the 60–120 s assumption.**
  - "You can't use a standby replica to serve read traffic." The standby is "synchronously replicated". Source: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/Concepts.MultiAZSingleStandby.html
  - Failover changes DNS: "you need to re-establish any existing connections". The page recommends a DNS TTL of 60 s or less.
- Multi-AZ DB *cluster* (one writer and two readable standbys, semisynchronous): "Failover times are typically under 35 seconds." Source: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/multi-az-db-clusters-concepts-failover.html
  - This option is limited to these classes: "`db.c6gd`, `db.m5d`, `db.m6gd`, `db.m6id`, `db.m6idn`, `db.m8gd`, `db.r5d`, ..." **db.m7g is not on the list.** Source: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/multi-az-db-clusters-concepts.html
  - I did not price the cluster option. UNVERIFIED.

### GCP Cloud SQL for PostgreSQL, Enterprise edition, Iowa (us-central1)
Source: https://cloud.google.com/sql/pricing (table "Enterprise edition - General Purpose machine series").

| Item | Rate | 2 vCPU / 8 GiB $/h | $/mo |
|---|---|---|---|
| vCPU / memory, zonal | $0.0413 per vCPU-h; $0.007 per GiB-h | 0.1386 | 101.18 |
| vCPU / memory, HA (regional) | $0.0826 per vCPU-h; $0.014 per GiB-h | 0.2772 | 202.36 |
| SSD storage, zonal / HA | $0.000232877 / $0.000465753 per GiB-h | – | ≈ $0.17 / $0.34 per GiB-month |
| Backups (used) | $0.000109589 per GiB-h | – | ≈ $0.08 per GiB-month |

From https://docs.cloud.google.com/sql/docs/postgres/high-availability:
- "When a failover occurs, you can expect the instance to be unavailable for about sixty seconds."
- "all writes made to the primary instance are replicated to disks in both zones before a transaction is reported as committed."
- "An HA-configured instance costs twice as much as a standalone instance."

---

## 3. Object storage and egress

| Item | Price | Source |
|---|---|---|
| S3 Standard, first 50 TB | $0.023 per GB-month (next 450 TB $0.022; over 500 TB $0.021) | feed `.../s3/USD/current/s3-standard.json`, behind https://aws.amazon.com/s3/pricing/ |
| S3 PUT, COPY, POST, LIST | $0.000005 per request = **$0.005 per 1,000** | same |
| S3 GET and other requests | $0.0000004 per request = **$0.0004 per 1,000** | same |
| AWS internet egress, us-east-1, first tier | **$0.09 per GB** for the first 10 TB/month; then $0.085 (next 40 TB), $0.07 (next 100 TB), $0.05 (over 150 TB) | feed `.../datatransfer/USD/current/datatransfer.json` |
| AWS free egress | "AWS customers receive 100GB of data transfer out to the internet free each month, aggregated across all AWS Services and Regions (except China and GovCloud)." | https://aws.amazon.com/s3/pricing/ |
| Cloudflare R2 Standard | "$0.015 / GB-month"; Class A "$4.50 / million requests"; Class B "$0.36 / million requests" | https://developers.cloudflare.com/r2/pricing/ |
| R2 free tier | "10 GB-month / month"; 1M Class A and 10M Class B requests per month | same |
| R2 egress | "There are no charges for egress bandwidth for any storage class." | same |
| R2 Infrequent Access | "$0.01 / GB-month", plus "$0.01 / GB" data retrieval | same |

---

## 4. TLS termination

**AWS Application Load Balancer, us-east-1.** Prices come from the feed `.../elb/USD/current/elb.json`, behind https://aws.amazon.com/elasticloadbalancing/pricing/.
- $0.0225 per ALB-hour, which is **$16.43/mo**.
- $0.008 per LCU-hour, which is **$5.84 per LCU-month**.
- LCU dimensions, from the same page; you are billed on the highest one:
  - 25 new connections per second
  - 3,000 active connections per minute (1,500 with mTLS)
  - 1 GB per hour processed for EC2, container and IP targets
  - 1,000 rule evaluations per second (the first 10 rules are free)
- WebSockets: "Application Load Balancers provide native support for WebSockets. ... You can use WebSockets with both HTTP and HTTPS listeners."
  - Caveat: "Websockets are not supported for requests routed to target groups that have enabled target optimizer."
  - Source: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/load-balancer-listeners.html

**Caddy** (https://caddyserver.com/docs/automatic-https):
- "Caddy was the first web server to use HTTPS automatically _and by default_."
- "By default, Caddy enables two ACME-compatible CAs" (Let's Encrypt and ZeroSSL). It falls back to the other CA if one fails.
- On-demand TLS "dynamically obtains a new certificate during the first TLS handshake that requires it". The page warns that this mode must be restricted to prevent abuse.
- Caddy itself costs nothing (open source). Certificates from Let's Encrypt cost nothing.

**Let's Encrypt rate limits** (https://letsencrypt.org/docs/rate-limits/, page "Last updated: August 5, 2026"):

| Limit | Value |
|---|---|
| Certificates per registered domain (or per IPv4, or per IPv6 /64) | 50 per 7 days, shared across accounts; override can be requested |
| Duplicate certificates (exact same identifier set) | "Up to 5 certificates can be issued per exact same set of identifiers every 7 days" |
| New orders per account | 300 per 3 hours |
| Authorization failures per identifier per account | 5 per hour |
| Consecutive authorization failures | 1,152; after that, issuance for the identifier is paused |
| New accounts per IPv4 | 10 per 3 hours |
| Renewals | ARI renewals are "exempt from all rate limits". Non-ARI renewals are exempt from the new-orders and per-domain limits only. |

---

## 5. Password hashing and authenticators

**OWASP Password Storage Cheat Sheet** (https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html):
- Argon2id: "Use Argon2id with a minimum configuration of 19 MiB of memory, an iteration count of 2, and 1 degree of parallelism." The sheet lists these equivalent configurations:
  - m=47104 (46 MiB), t=1, p=1
  - **m=19456 (19 MiB), t=2, p=1**
  - m=12288 (12 MiB), t=3, p=1
  - m=9216 (9 MiB), t=4, p=1
  - m=7168 (7 MiB), t=5, p=1
- scrypt, as a fallback. The sheet lists these equivalent configurations:
  - **N=2^17 (128 MiB), r=8, p=1**
  - N=2^16, r=8, p=2
  - N=2^15, r=8, p=3
  - N=2^14, r=8, p=5
  - N=2^13, r=8, p=10
- bcrypt: work factor "minimum of 10". "bcrypt has a maximum length input length of 72 bytes".
- PBKDF2: "PBKDF2-HMAC-SHA256: 600,000 iterations"; "PBKDF2-HMAC-SHA512: 220,000 iterations".

**NIST SP 800-63B** (SP 800-63-4 suite; https://pages.nist.gov/800-63-4/sp800-63b.html; the page states that it supersedes the earlier 800-63B; page stamp 26 Aug 2025):
- Length:
  - "Verifiers and CSPs SHALL require passwords that are used as a single-factor authentication mechanism to be a minimum of 15 characters in length."
  - Passwords used only as part of MFA: "a minimum of eight characters".
  - "SHOULD permit a maximum password length of at least 64 characters."
- "SHALL NOT impose other composition rules (e.g., requiring mixtures of different character types)"
- "SHALL NOT require subscribers to change passwords periodically. However, verifiers SHALL force a change if there is evidence that the authenticator has been compromised."
- "SHALL compare the prospective secret against a blocklist that contains known commonly used, expected, or compromised passwords."
- Storage:
  - "Passwords SHALL be salted and hashed using a suitable password hashing scheme."
  - "The chosen cost factor SHOULD be as high as practical without negatively impacting verifier performance."
  - The document points to SP 800-132 for approved schemes. It does not name Argon2.
- Phishing resistance: "Verifiers SHALL offer at least one phishing-resistant authentication option at AAL2". WebAuthn is cited as an example of phishing resistance through verifier name binding.
- Syncable authenticators (passkeys):
  - "Syncable authenticators may be offered as a new option for applications that seek to implement up to AAL2."
  - "syncable authenticators SHALL NOT be used at AAL3."
  - The word "passkey" does not appear in the page text. The term used is "syncable authenticators".

---

## 6. Auth prior art: sharing models

**Onshape** (https://cad.onshape.com/help/Content/Collaboration/share_documents.htm):
- Permission levels:
  - "Can edit": "Permission to edit, copy, link, export, share, and comment on a document."
  - "Can view": "Permission to view, copy, link, export, and comment on a document."
  - Optional checkboxes: Copy, Link document (requires Export), Export, Share, Comment, Delete.
- Who can share: only the owner and users with "Can edit & share". "Ownership is the highest level of permissions". Ownership is transferable to a user or to a company.
- Link sharing is view-only, for one document, and "does not require signing in to Onshape for viewing". Export through the link is optional.
- Groups: "Select a team to send a share message to all members of that team". Sharing with a company works the same way. "enterprise documents cannot be public."
- **There is no edit-via-link.**

**Figma**, from "File and folder permissions" (https://help.figma.com/hc/en-us/articles/35361119554711; no last-updated date on the page):
- Roles:
  - Owner: "The person who created the file. They have full editing access."
  - Can edit: "Full file editing access."
  - Can view: "View and comment on the file, but not edit it."
- Viewing users can comment.
- Editing also requires a paid seat for that product.
- Inheritance:
  - Files always inherit roles from their folder.
  - Top-level folders follow team permissions on Organization and Enterprise plans.
  - A file can grant more access than its folder role, but not less.

**Figma**, from "Share files and prototypes" (https://help.figma.com/hc/en-us/articles/360040531773):
- Audience options:
  - "Anyone, even those outside your organization"
  - "Anyone at your organization" (Organization and Enterprise plans)
  - The workspace (Enterprise plan)
  - "Only those directly invited to the file can access it"
- "What they can do": view or edit. **A link can grant edit.**
- Password-protected links on paid plans; link expiration on Enterprise only.

---

## 7. Postgres facts (PostgreSQL 17 docs)

**synchronous_commit** (https://www.postgresql.org/docs/17/runtime-config-wal.html):
- "If synchronous_standby_names is empty, the only meaningful settings are on and off; remote_apply, remote_write and local all provide the same local synchronization level as on."
- `remote_apply`: "commits will wait until replies from the current synchronous standby(s) indicate they have received the commit record of the transaction and applied it, so that it has become visible to queries on the standby(s), and also written to durable storage on the standbys."
- `on` (the default): "commits wait until replies from the current synchronous standby(s) indicate they have received the commit record of the transaction and flushed it to durable storage. This ensures the transaction will not be lost unless both the primary and all synchronous standbys suffer corruption of their database storage."
- `remote_write`: the standby has "written it to their file systems. This setting ensures data preservation if a standby instance of PostgreSQL crashes, but not if the standby suffers an operating-system-level crash".
- `local`: waits "for local flush to disk, but not for replication".
- The setting can be changed per transaction.

**FOR UPDATE SKIP LOCKED** (https://www.postgresql.org/docs/17/sql-select.html):
- "With SKIP LOCKED, any selected rows that cannot be immediately locked are skipped. Skipping locked rows provides an inconsistent view of the data, so this is not suitable for general purpose work, but can be used to avoid lock contention with multiple consumers accessing a queue-like table."

**LISTEN/NOTIFY** (https://www.postgresql.org/docs/17/sql-notify.html, https://www.postgresql.org/docs/17/sql-listen.html):
- Recipients: NOTIFY sends the event "to each client application that has previously executed LISTEN channel for the specified channel name in the current database." Sessions that are not listening do not receive it. The docs define no durable delivery (my reading).
- Payload: "In the default configuration it must be shorter than 8000 bytes. (If binary data or large amounts of information need to be communicated, it's best to put it in a database table and send the key of the record.)"
- Delivery at commit: "if a NOTIFY is executed inside a transaction, the notify events are not delivered until and unless the transaction is committed."
- Delivery to the client: "notification events are only delivered between transactions" on the listening side.
- Duplicates within one transaction: "If the same channel name is signaled multiple times with identical payload strings within the same transaction, only one instance of the notification event is delivered".
- Ordering: "messages from different transactions are delivered in the order in which the transactions committed."
- Queue: "If this queue becomes full, transactions calling NOTIFY will fail at commit. The queue is quite large (8GB in a standard installation)".
- Race when setting up LISTEN: "first execute (and commit!) that command, then in a new transaction inspect the database state ..., then rely on notifications".

**PITR** (https://www.postgresql.org/docs/17/continuous-archiving.html, https://www.postgresql.org/docs/17/app-pgbasebackup.html):
- "this technique supports point-in-time recovery: it is possible to restore the database to its state at any time since your base backup was taken."
- "you need a continuous sequence of archived WAL files that extends back at least as far as the start time of your backup."
- To enable archiving: set `wal_level` to `replica` or higher, `archive_mode = on`, and `archive_command` or `archive_library`.
- "The easiest way to perform a base backup is to use the pg_basebackup tool."
- pg_basebackup "can be used both for point-in-time recovery ... and as the starting point for a log-shipping or streaming-replication standby server". In v17 it "can take a full or incremental base backup".

---

### Not verified / gaps
- GCP prices for us-east1 or us-east4: I captured only the default us-central1 rows. UNVERIFIED.
- Hetzner USD prices: UNVERIFIED (the pages show EUR only).
- The unit of Hetzner's extra-traffic charge, which the page prints as "per hour": UNVERIFIED.
- Pricing for the RDS Multi-AZ DB cluster option: UNVERIFIED.
- AWS and GCP spot prices are point-in-time snapshots taken 2026-11-25. They move daily.
