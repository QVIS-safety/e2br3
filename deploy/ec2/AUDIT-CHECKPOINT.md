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

The current AWS login succeeds, but all 11 visible buckets deny
`s3:GetBucketObjectLockConfiguration`; the user has confirmed they cannot grant the missing access.
No existing bucket is therefore approved or known to satisfy Object Lock. An AWS administrator must:

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

### Current enrollment blocker

The read-only production-data clone check found 34 existing `prev_hash` link failures among 6,327,905
rows (633 pages), first at audit ID 13788833. The same 34 failures exist in the original database, while
their stored row payload hashes verify; the affected rows have `created_at` dates of 2026-10-17 and
2026-10-20. The preserved graph confirms 17 historical forks: each affected row is a leaf, its stored
predecessor has both that leaf and the next ID as children, and the following row skips the leaf. This
matches the reproduced legacy concurrency mechanism: concurrent inserts received IDs before serialization,
then acquired the hash-chain lock in a different order. A four-row legacy-writer reproduction produced two
broken links where zero were expected, while the current writer passed all 10 database tests. The private
evidence is retained in `historical-fork-graph.json`, `legacy-order.log`, and `database.log`. The authorized
qualified disposition below preserves those failures and starts a separately reported version-2 epoch; it
does not repair or authenticate the earlier history.
Preserve the query results and affected rows as evidence. Do not enroll the current chain as healthy, and
do not rehash, backfill, suppress, or bypass these failures to manufacture a pass. Enrollment requires
authentic recovery or the authorized qualified disposition below, which preserves the historical evidence.
S3 Object Lock inspection also remains blocked by `AccessDenied` as documented above.

### Qualified post-boundary enrollment

The opt-in format-2 checkpoint preserves the legacy failures instead of treating them as valid. Its
retained disposition says `historical_status: "known_broken_preserved"`, embeds the exact ordered defect
records and their digest, the full version-1 canonical baseline, cutoff ID/hash, root-cause statement,
evidence SHA-256 and migration SHA-256. Every run recomputes every version-1 payload hash, baseline and
link defect, requires the exact retained defect set, rejects version interleaving across the cutoff, and
verifies a nonempty continuous version-2 suffix linked through the cutoff row. Results always use
`verified: false` and report `post_boundary_verified: true`; they never describe the historical chain as
healthy. The ordinary format-1 mode remains strict and continues to reject the existing failures. The
database verifier and audit API are not relaxed and continue to report the preserved historical breaks.

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
VersionIds, the second object's prior link, and a forced stale concurrent writer failure. Production
verification is pending: the AWS login succeeds, but the current identity cannot inspect Object Lock on
any of the 11 visible buckets and no approved checkpoint bucket or dedicated writer is available.
