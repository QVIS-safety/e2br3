#!/usr/bin/env python3
"""Deploy CI-published dev images into the existing staging Compose project."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time
from urllib.parse import unquote, urlparse


IMAGES = {
    "backend": "ghcr.io/qvis-safety/e2br3-web-server:dev",
    "frontend": "ghcr.io/qvis-safety/e2br3-frontend-dev:dev",
}


def migration_plan(expected, applied):
    unknown = sorted(set(applied) - set(expected))
    changed = sorted(name for name in applied.keys() & expected.keys()
                     if applied[name] != expected[name])
    if unknown or changed:
        raise ValueError(f"Incompatible migration history: unknown={unknown}, changed={changed}")
    return sorted(set(expected) - set(applied))


def image_reference(image, repository):
    labels = image["Config"]["Labels"]
    if labels["io.qvis.staging.ci"] != "passed":
        raise ValueError("Image was not published through staging CI")
    if re.fullmatch(r"[0-9a-f]{40}", labels["org.opencontainers.image.revision"]) is None:
        raise ValueError("Missing exact image source revision")
    refs = [ref for ref in image["RepoDigests"] if ref.startswith(repository + "@sha256:")]
    if len(refs) != 1 or re.fullmatch(re.escape(repository) + r"@sha256:[0-9a-f]{64}", refs[0]) is None:
        raise ValueError("Expected one immutable registry digest")
    return refs[0]


def write_json(path, value):
    temporary = path.with_suffix(path.suffix + ".next")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.chmod(0o600)
    temporary.replace(path)


def deploy(compose_file, state_dir, check_only):
    def run(args, **kwargs):
        result = subprocess.run(args, capture_output=True, text=True, timeout=900, **kwargs)
        if result.returncode:
            log = state_dir / "last-error.log"
            log.write_text(result.stdout + result.stderr)
            log.chmod(0o600)
            raise RuntimeError(f"{args[0]} failed ({result.returncode}); see {log}")
        return result.stdout

    compose = ["docker", "compose", "-f", str(compose_file)]
    config = json.loads(run(compose + ["config", "--format", "json"]))
    if config["name"] != "safetydb-staging":
        raise ValueError("Only the safetydb-staging Compose project may be updated")
    environment = config["services"]["backend"]["environment"]
    for key, value in {"SKIP_DEV_INIT": "1", "E2BR3_DEV_DB_INIT_ENABLED": "false",
                       "SERVICE_BOOTSTRAP_ENABLED": "false"}.items():
        if environment[key] != value:
            raise ValueError(f"Unsafe database initialization setting: {key}")
    database = urlparse(environment["SERVICE_MIGRATION_DB_URL"])
    runtime_database = urlparse(environment["SERVICE_DB_URL"])
    if (database.hostname, database.path) != (runtime_database.hostname, runtime_database.path):
        raise ValueError("Runtime and migration database targets differ")
    if not all([database.hostname, database.username, database.password, database.path]) \
            or database.username == runtime_database.username:
        raise ValueError("A separate, explicit migration credential is required")
    pg_env = dict(os.environ, PGHOST=database.hostname, PGUSER=unquote(database.username),
                  PGPASSWORD=unquote(database.password), PGDATABASE=database.path[1:],
                  PGSSLMODE="require", PGCONNECT_TIMEOUT="10")
    if database.port is not None:
        pg_env["PGPORT"] = str(database.port)

    previous = {"services": {}}
    desired = {"services": {}}
    changed = []
    revisions = {}
    for service, tag in IMAGES.items():
        current = json.loads(run(["docker", "inspect",
                                  config["services"][service]["container_name"]]))[0]
        if not current["State"]["Running"]:
            raise ValueError(f"Current {service} is not running; repair it before automatic deployment")
        previous["services"][service] = {"image": current["Image"]}
        run(["docker", "pull", tag])
        image = json.loads(run(["docker", "image", "inspect", tag]))[0]
        ref = image_reference(image, tag.rsplit(":", 1)[0])
        desired["services"][service] = {"image": ref}
        revisions[service] = image["Config"]["Labels"]["org.opencontainers.image.revision"]
        if image["Id"] != current["Image"]:
            changed.append(service)

    with tempfile.TemporaryDirectory(prefix="candidate-", dir=state_dir) as directory:
        candidate = Path(directory)
        container = run(["docker", "create", desired["services"]["backend"]["image"]]).strip()
        try:
            run(["docker", "cp", container + ":/app/db", str(candidate / "db")])
            run(["docker", "cp", container + ":/app/deploy", str(candidate / "deploy")])
        finally:
            run(["docker", "rm", container])
        expected = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                    for p in (candidate / "db/migrations").glob("*.sql")}
        if not expected:
            raise ValueError("Candidate image contains no migrations")
        query = "SELECT json_object_agg(filename, checksum) FROM public.schema_migrations;"
        applied = json.loads(run(["psql", "-XAt", "-v", "ON_ERROR_STOP=1", "-c", query], env=pg_env))
        if not isinstance(applied, dict) or not applied:
            raise ValueError("Existing database needs an explicitly verified migration baseline")
        pending = migration_plan(expected, applied)
        plan = {"revisions": revisions, "changed": changed, "pending_migrations": pending,
                "database": database.path[1:]}
        print(json.dumps(plan), flush=True)
        if check_only:
            write_json(state_dir / "preflight.json", plan)
            return
        if pending:
            backup = state_dir / ("database-before-" + str(time.time_ns()) + ".dump")
            run(["pg_dump", "--format=custom", "--file", str(backup)], env=pg_env)
            backup.chmod(0o600)
            run(["pg_restore", "--list", str(backup)])
            migration_env = dict(os.environ, DATABASE_URL=environment["SERVICE_MIGRATION_DB_URL"],
                                 PROJECT_DIR=str(candidate), BASELINE="0", BASELINE_IF_EMPTY="0")
            run(["sh", str(candidate / "deploy/ec2/migrate-rds.sh")], env=migration_env)

    if not changed:
        write_json(state_dir / "last-success.json", dict(plan, status="UNCHANGED"))
        return

    override = state_dir / "images.json"
    write_json(state_dir / "previous-images.json", previous)
    write_json(override, desired)
    deployment = compose + ["-f", str(override), "up", "-d", "--no-deps", "--pull", "never", "--no-build"]

    def health():
        checks = [("http://127.0.0.1:8216/health", "204"),
                  ("http://127.0.0.1:8080/health", "204"),
                  ("http://127.0.0.1:4033/", "200"),
                  ("http://127.0.0.1:4033/api/cases", "403"),
                  ("https://safety-staging.qvis.co.kr/health", "204")]
        for attempt in range(24):
            healthy = True
            for url, expected_status in checks:
                response = subprocess.run(["curl", "--silent", "--show-error", "--connect-timeout", "3",
                                           "--max-time", "5", "--output", "/dev/null", "--write-out",
                                           "%{http_code}", url], capture_output=True, text=True, timeout=10)
                if response.returncode or response.stdout != expected_status:
                    healthy = False
            if healthy:
                return
            if attempt < 23:
                time.sleep(5)
        raise RuntimeError("Staging HTTP/API proxy health verification failed")

    try:
        run(deployment + changed)
        health()
    except Exception:
        # Schema changes remain applied; restoring an image is never a database rollback.
        write_json(override, previous)
        run(deployment + changed)
        health()
        write_json(state_dir / "last-failure.json", dict(plan, status="IMAGES_RESTORED"))
        raise
    write_json(state_dir / "last-success.json", dict(plan, status="DEPLOYED", images=desired))
    print("Staging deployment verified", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compose-file", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    if not args.compose_file.is_absolute() or not args.state_dir.is_absolute():
        parser.error("compose-file and state-dir must be absolute paths")
    os.umask(0o077)
    args.state_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    args.state_dir.chmod(0o700)
    with (args.state_dir / "deploy.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        deploy(args.compose_file, args.state_dir, args.check_only)
