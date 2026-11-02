# Independent audit checkpoints

This command verifies the database audit chain and writes the verified head to an existing S3 bucket
with Object Lock. It never creates a bucket, IAM identity, policy, schedule, or local fallback.

Provision these production controls separately:

- a versioned bucket with Object Lock enabled and a retention period approved by the record owner;
- a dedicated writer role/profile, separate from the application role, allowed to read the checkpoint
  prefix, read retention/version metadata, and create retained objects, but not bypass governance;
- a database checkpoint reader that can read the complete global `audit_logs` table despite tenant RLS;
  an ordinary application tenant connection cannot produce the canonical legacy baseline;
- an operator/verifier identity that can read all object versions, delete markers, metadata and retention;
- the expected AWS account ID, exact writer ARN, stable database UUID, and actual PostgreSQL database name.

Have the record owner approve the COMPLIANCE retention duration before provisioning; pass that exact
number as `--retention-days`. Do not infer it from a development environment. Create a new Object-Lock
enabled bucket if no approved existing bucket has Object Lock; Object Lock cannot be enabled by this
script. The dedicated writer's least-privilege policy can use this shape after replacing both placeholders:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetBucketObjectLockConfiguration",
        "s3:GetBucketVersioning"
      ],
      "Resource": "arn:aws:s3:::<checkpoint-bucket>"
    },
    {
      "Effect": "Allow",
      "Action": "s3:ListBucketVersions",
      "Resource": "arn:aws:s3:::<checkpoint-bucket>",
      "Condition": {
        "StringLike": {"s3:prefix": "checkpoints/<database-uuid>/*"}
      }
    },
    {
      "Effect": "Allow",
      "Action": [
        "s3:GetObjectVersion",
        "s3:GetObjectRetention",
        "s3:PutObject",
        "s3:PutObjectRetention"
      ],
      "Resource": "arn:aws:s3:::<checkpoint-bucket>/checkpoints/<database-uuid>/*"
    }
  ]
}
```

Do not grant delete, Object Lock bypass, bucket-policy, or retention-shortening permissions. Use a
dedicated IAM user or role whose only operational purpose is checkpoint publication; a shared human
login and the application runtime identity do not satisfy the identity-separation requirement. Bucket
policy must deny non-TLS access and checkpoint-prefix writes by every principal except the dedicated
writer according to the organization's access process. The application and database administrator must
not have the writer credentials or permission to assume that identity. The object's `writer-arn` metadata
is only an expected-label consistency check because the writer supplies it; the exclusive-writer bucket
policy establishes who can write, and CloudTrail data events provide evidence of the actual writing
principal. Enable and retain those data events under the organization's audit policy.

## Administrator handoff

The initial 2026-11-02 inspection denied `s3:GetBucketObjectLockConfiguration` on all 11 visible
buckets. That is a historical result, not evidence of current permissions after subsequent access
changes. No successful Object Lock enrollment has been recorded. Before enrollment, an administrator must:

1. choose or create an Object-Lock-enabled, versioned bucket and its owning account;
2. obtain the record owner's COMPLIANCE retention duration in days;
3. create a dedicated checkpoint IAM user or role and attach the scoped policy above;
4. provide the bucket, account ID, region, profile/role ARN, and approved retention days to the operator;
5. let the operator run the documented preflight and two-run verification before scheduling.

These are prerequisites, not actions performed by this repository or script.

## Database rollout order

The checkpoint command depends on the `hash_version` column and bounded range verifier introduced by
`db/migrations/20261102_hash_chain_integrity.sql`. Apply that migration transactionally under the production
maintenance policy before replacing the backend binary. The migration preserves existing rows as version
1 and does not rehash their stored `prev_hash` or `entry_hash`; new audit rows use version 2. Adding the
metadata column and replacing trigger/functions still take PostgreSQL DDL locks, so schedule the operation
using the measured maintenance window rather than treating it as an online no-impact change.

The migration adds `audit_logs_hash_version_valid` as `NOT VALID` so the 15 GB table is not fully scanned
while the adding transaction holds its stronger DDL lock. After that migration transaction commits, run
constraint validation in a separate committed transaction; do not fold it back into the migration's add
transaction:

```sql
BEGIN;
ALTER TABLE public.audit_logs
  VALIDATE CONSTRAINT audit_logs_hash_version_valid;
COMMIT;
```

Require `convalidated = true` for that constraint in `pg_constraint` before backend rollout or checkpoint
enrollment. New inserts and updates are constrained while `NOT VALID`; validation establishes coverage of
the existing version-1 rows. See PostgreSQL's official
[`ALTER TABLE`](https://www.postgresql.org/docs/18/sql-altertable.html) documentation for the distinct lock
behavior of adding a not-valid constraint and validating it later.

After migration, verify that existing row counts and stored version-1 hashes are unchanged, that the
bounded verifier completes from genesis through a frozen head, and that newly appended rows use hash
version 2. Confirm `audit_logs_hash_version_valid` is validated, then replace the backend with the matching
build and perform the independent checkpoint enrollment below. Do not enroll a baseline before the
migration or report this sequence as deployed until those production checks pass.

### Historical link failures and authorized removal (2026-11-02)

The read-only production-data clone check found 34 existing `prev_hash` link failures among 6,327,905
rows (633 pages), first at audit ID 13788833. The same 34 failures were confirmed in the original database,
while their stored row payload hashes verified; the affected rows have `created_at` dates of 2026-10-17 and
2026-10-20. The preserved graph confirms 17 historical forks: each affected row is a leaf, its stored
predecessor has both that leaf and the next ID as children, and the following row skips the leaf. This
matches the reproduced legacy concurrency mechanism: concurrent inserts received IDs before serialization,
then acquired the hash-chain lock in a different order. A four-row legacy-writer reproduction produced two
broken links where zero were expected, while the current writer passed all 10 database tests. The private
evidence is retained in `historical-fork-graph.json`, `legacy-order.log`, and `database.log`. The initial
qualified disposition preserved those failures; it remains evidence of the state before removal.

Under subsequent explicit user authorization, a one-time cleanup was applied **only** to the Oracle AS2
database `e2br3_ui_as2_transport_20260924`. The 34 reported failures comprised 17 late leaf rows and their
17 valid successors. Deleting all 34 would also remove a signature-linked audit row and cause 17 new
link failures. After a fresh full database backup and an exact private export, only the 17 leaves were
removed. They had no hash children or electronic-signature references. The 17 successors, including the
signature-linked row, matched the preserved original payloads after cleanup. Surviving hashes were not
rewritten, and business records and electronic signatures were preserved.

The guarded maintenance transaction recorded and verified:

- 6,328,613 audit rows before cleanup, minus 17 removed rows, plus one version-2 maintenance event;
- all 6,328,597 resulting rows across 633 pages: 6,328,298 version-1 rows and 299 version-2 rows,
  with zero payload-hash or predecessor-link failures before commit;
- maintenance audit ID `14161459`, table `audit_log_maintenance`, operation
  `4f05c410-8557-44a1-a84b-f3b3939e34ec`, recording authorization and archive/backup digests;
- frozen head hash `f9a66d09ac3ff157e89186144666907f524d148277ec24ed2b80f93aa5495325`;
- all four audit protection triggers restored and direct application INSERT/UPDATE/DELETE/TRUNCATE
  privileges denied. Postcommit authenticated API checks passed all 17 affected regions and the then-current
  301-row version-2 suffix through ID `14161461`; API health was 204 and frontend health was 200.

Private evidence is retained on the Oracle host under `/home/opc/e2br3/audit-prune-20261102/`:
`result.json`, `execution-result.json`, `postflight-db.private.json`, `postflight-api.private.json`, the
exact removed-row archive and full database backup. The removed-row archive SHA-256 is
`124e64358b10984d1e4ce0525ed3dc2ec2f27c994031cd79135e34302d3f3ba8`. Original 67-row forensic evidence
and earlier fullscan results remain unchanged under `audit-integrity-20261102/`. Raw rows, credentials,
backup contents and the private one-time execution script are not repository artifacts.

This verifies the surviving chain, not the completeness or authenticity of the original history:
17 audit rows were deliberately removed. The old canonical baseline and qualified disposition describe
the pre-removal state and must not be reused or edited to conceal that difference. Any future enrollment
of this database must derive a fresh baseline from its actual state and retain the cleanup authorization
and evidence provenance alongside the independent checkpoint. S3 enrollment has not been performed.
The backup archive was fully readable; a full database restore rehearsal was not performed for this cleanup.

This was database maintenance, not a migration for other databases. Pushing this document does not repeat
the deletion on EC2 or another database. A database retaining the original failures must still fail strict
verification; do not suppress errors, rewrite hashes, or reuse the target IDs as a generic repair.

### Qualified post-boundary enrollment

For a database retaining documented legacy failures, the opt-in format-2 checkpoint preserves those
failures instead of treating them as valid. It does not apply the cleanup above. Its
retained disposition says `historical_status: "known_broken_preserved"`, embeds the exact ordered defect
records and their digest, the full version-1 canonical baseline, cutoff ID/hash, root-cause statement,
evidence SHA-256 and migration SHA-256. Every run recomputes every version-1 payload hash, baseline and
link defect, requires the exact retained defect set, rejects version interleaving across the cutoff, and
verifies a nonempty continuous version-2 suffix linked through the cutoff row. Results always use
`verified: false` and report `post_boundary_verified: true`; they never describe the historical chain as
healthy. The ordinary format-1 mode remains strict and rejects any existing failures. The database verifier
and audit API are not relaxed. Do not supply the pre-removal `--legacy-disposition` for the cleaned AS2
database; its actual rows no longer match that disposition.

After migration and at least one version-2 audit append, an operator can generate the private manifest
through the same bounded, repeatable-read path without printing row payloads. The helper creates a new
0600 file and refuses to overwrite one:

```python
from types import SimpleNamespace
from scripts import audit_checkpoint

summary = audit_checkpoint.prepare_disposition(
    SimpleNamespace(
        psql="psql",
        database_uuid="<stable-database-uuid>",
        database_name="<actual-database-name>",
    ),
    cutoff_id=<last-version-1-id>,
    root_cause="<evidence-backed disposition>",
    evidence_sha256="<64-lowercase-hex>",
    migration_sha256="<64-lowercase-hex>",
    path="<private-new-disposition.json>",
)
print(summary)  # IDs, hashes and counts only; no clinical payload
```

Set `AUDIT_CHECKPOINT_DATABASE_URL` as for normal verification. Review the private manifest against the
preserved evidence, then pass it exactly once with `--bootstrap --legacy-disposition <path>`. Later runs
load the immutable disposition from S3 and reject a command-line override or format downgrade. Preparing
the local file is not independent proof or S3 enrollment; only a successfully retained and read-back
Object Lock checkpoint establishes the external anchor.

## Trust boundary

This tool trusts its own host and code, the PostgreSQL engine, and the deployed verifier functions. The
retained S3 head detects audit-row tail deletion, rollback, or rewrite when PostgreSQL and the verifier
report truthfully. It is not cryptographic attestation of a compromised database server: a database
superuser who can replace verifier functions or control query responses can evade database-side version-2
checks. The version-1 baseline reads canonical rows directly, but still trusts the engine to return those
rows. Keep database superuser access and checkpoint-host administration outside the application role and
review them through the organization's privileged-access controls.

The legacy-row query is a top-level `SELECT` and sets psql `FETCH_COUNT=1000` so psql retrieves it through
a cursor instead of buffering the full result in memory. A top-level `WITH` command is not used here:
actual-volume testing showed psql buffered that form instead of streaming it. See the official psql
[`FETCH_COUNT`](https://www.postgresql.org/docs/18/app-psql.html) documentation.

The prefix must contain only sequence-named checkpoint objects. Every object uses COMPLIANCE retention,
links the prior object's version and body hash, and is created conditionally. The command rejects deleted,
overwritten, missing, expired, foreign-writer, broken-chain, stale-writer, database rollback, legacy-hash,
partial-range, empty-database, and identity-mismatch states. Existing version-1 rows are enrolled once as
an independently hashed canonical stream (including organization, stored hashes and all protected fields)
and fully rechecked on every run. This proves they have not changed after enrollment; it does not prove
their history before enrollment. All hash-chain rows from genesis are also reverified on every run. A
missing prefix never auto-bootstraps.

The verifier requires active retention on every historical checkpoint, not only the newest object.
Before the oldest retention date expires, the operator must extend retention on every affected version
under the record owner's approved policy. If any historical version expires, verification fails clearly;
the command does not accept an expired chain or shorten retention. Retention renewal is an operational
procedure and is not automated here.

Checkpoint writes set an explicit SHA-256 request checksum because S3 requires an integrity header for
Object Lock writes. See the primary AWS documentation for
[PutObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html) and
[Object Lock retention](https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lock-managing.html).

Set the database URL without putting credentials on the command line, then bootstrap an empty prefix once:

```sh
export AUDIT_CHECKPOINT_DATABASE_URL='postgresql://...'
python3 scripts/audit_checkpoint.py --bootstrap \
  --bucket '<object-lock-bucket>' --bucket-owner '<12-digit-account-id>' \
  --region '<region>' --profile '<dedicated-writer-profile>' \
  --writer-arn 'arn:aws:iam::<account-id>:role/<checkpoint-writer>' \
  --database-uuid '<stable-uuid>' --database-name '<actual-database-name>' \
  --retention-days '<approved-days>'
```

Remove `--bootstrap` on later runs. Add `--verify-only` for a read-only verification that never publishes.
An unchanged database head returns a successful `unchanged` result without creating a redundant object.
Run the publishing command from the production scheduler and alert on every
nonzero exit; absence of a fresh success is a checkpoint failure. Keep the printed JSON as operational
evidence, but treat it as valid only for that invocation. The application/API must return
`checkpoint_status: "not_checked"`; it must not cache this CLI's PASS or accept a client-supplied head as
independent proof.

Before enabling the schedule, use the verifier identity to inspect `list-object-versions` and
`get-object-retention` for the prefix, confirm the bucket owner and writer ARN, and perform two real runs
with a real intervening audit append. Confirm sequences 1 and 2, active COMPLIANCE retention, distinct
VersionIds, the second object's prior link, and a forced stale concurrent writer failure. Independent
enrollment and this two-run verification remain unperformed for the cleaned AS2 database. Check the actual
AWS identity, approved bucket, dedicated writer and retention configuration before attempting enrollment;
the historical inspection result above does not establish their current state.
