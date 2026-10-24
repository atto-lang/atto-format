#!/usr/bin/env python3
"""Read-only fail-closed CI receipt for an exact private formatter Git tree.

Only validates GitHub job metadata; never replaces compilation, test logs,
compiler parity, or independent authorization for cross-organization source.
"""
import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request

MAX_RESPONSE_BYTES = 2 * 1024 * 1024
REPO_NAME = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*\Z", re.ASCII)
SHA = re.compile(r"[0-9a-f]{40}\Z", re.ASCII)
TOKEN = re.compile(r"[A-Za-z0-9._~+/-]+={0,16}\Z", re.ASCII)
REQUIRED_STEPS = {
    "test": ("cargo fmt --all", "cargo test --all-targets", "cargo test --doc",
             "cargo clippy --all-targets", "cargo build --release --bins"),
    "msrv": ("cargo test --all-targets",),
}

class Denied(Exception):
    pass

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, url):
        raise Denied("unexpected metadata redirect")


def fetch_json(path, token):
    if not isinstance(token, str) or not (1 <= len(token) <= 4096) or not TOKEN.fullmatch(token):
        raise Denied("authenticated GitHub metadata required")
    if not path.startswith("/repos/") or "?" in path and not path.endswith("?per_page=100"):
        raise Denied("unexpected API path")
    url = "https://api.github.com" + path
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json", "Authorization": "Bearer " + token,
        "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "oresfmt-ci-evidence"},
        method="GET")
    try:
        # Avoid inherited HTTP(S)_PROXY carrying a metadata-read bearer token.
        with urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect()).open(
            request, timeout=10
        ) as response:
            if response.status != 200 or response.geturl() != url:
                raise Denied("invalid GitHub metadata response")
            raw = response.read(MAX_RESPONSE_BYTES + 1)
    except (urllib.error.URLError, OSError, TimeoutError, ValueError):
        raise Denied("GitHub metadata unavailable") from None
    if len(raw) > MAX_RESPONSE_BYTES:
        raise Denied("GitHub metadata response too large")
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeError, ValueError):
        raise Denied("invalid GitHub metadata JSON") from None


def verify(repo, repo_id, head, tree, run_id, token, fetch=fetch_json):
    if not isinstance(repo, str) or not REPO_NAME.fullmatch(repo) or any(
        p in (".", "..") for p in repo.split("/")
    ):
        raise Denied("invalid repository name")
    if type(repo_id) is not int or repo_id <= 0 or type(run_id) is not int or run_id <= 0:
        raise Denied("positive integer identity required")
    if not isinstance(head, str) or not SHA.fullmatch(head):
        raise Denied("invalid exact source commit SHA")
    if not isinstance(tree, str) or not SHA.fullmatch(tree):
        raise Denied("invalid exact source tree SHA")
    if not isinstance(token, str) or not (1 <= len(token) <= 4096) or not TOKEN.fullmatch(token):
        raise Denied("authenticated GitHub metadata required")

    prefix = "/repos/" + repo
    identity = fetch(prefix, token)
    if not isinstance(identity, dict) or identity.get("id") != repo_id or type(identity.get("id")) is not int \
        or str(identity.get("full_name", "")).casefold() != repo.casefold() \
        or identity.get("private") is not True or identity.get("visibility") != "private" \
        or identity.get("archived") is not False or identity.get("disabled") is not False:
        raise Denied("source repository identity/private status invalid")

    commit = fetch(prefix + "/git/commits/" + head, token)
    if not isinstance(commit, dict) or commit.get("sha") != head \
        or not isinstance(commit.get("tree"), dict) or commit["tree"].get("sha") != tree:
        raise Denied("exact source commit/tree mismatch")

    run = fetch(prefix + "/actions/runs/" + str(run_id), token)
    if not isinstance(run, dict) or run.get("head_sha") != head \
        or run.get("status") != "completed" or run.get("conclusion") != "success" \
        or run.get("event") not in ("pull_request", "push", "workflow_dispatch") \
        or type(run.get("run_attempt")) is not int or run["run_attempt"] < 1 \
        or not isinstance(run.get("repository"), dict) \
        or type(run["repository"].get("id")) is not int \
        or run["repository"]["id"] != repo_id:
        raise Denied("CI run not completed successfully on exact source head")

    jobs_data = fetch(prefix + "/actions/runs/" + str(run_id) + "/jobs?per_page=100", token)
    if not isinstance(jobs_data, dict) or not isinstance(jobs_data.get("jobs"), list) \
        or type(jobs_data.get("total_count")) is not int \
        or jobs_data["total_count"] != len(jobs_data["jobs"]):
        raise Denied("missing/paginated job evidence")
    jobs = {}
    for job in jobs_data["jobs"]:
        if not isinstance(job, dict):
            raise Denied("malformed CI job")
        name = job.get("name")
        if name in REQUIRED_STEPS:
            if name in jobs:
                raise Denied("duplicate required CI job")
            jobs[name] = job
    if set(jobs) != set(REQUIRED_STEPS):
        raise Denied("missing required CI jobs")

    for name, required_steps in REQUIRED_STEPS.items():
        job = jobs[name]
        if job.get("status") != "completed" or job.get("conclusion") != "success" \
            or type(job.get("runner_id")) is not int or job["runner_id"] <= 0 \
            or not isinstance(job.get("runner_name"), str) or not job["runner_name"] \
            or job.get("run_attempt") != run["run_attempt"] \
            or not isinstance(job.get("steps"), list) or len(job["steps"]) == 0:
            raise Denied("required CI job has no successful executed runner evidence: " + name)
        executed_names = []
        for step in job["steps"]:
            if not isinstance(step, dict) or step.get("status") != "completed" \
                or step.get("conclusion") != "success" or not isinstance(step.get("name"), str):
                raise Denied("required CI job has failed/unexecuted steps: " + name)
            executed_names.append(step["name"])
        for required in required_steps:
            if not any(required in executed for executed in executed_names):
                raise Denied("required CI command did not execute: " + name + " / " + required)
    return {"repo": repo, "commit": head, "tree": tree, "run_id": run_id,
            "run_attempt": run["run_attempt"], "jobs": sorted(jobs)}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--repo-id", required=True, type=int)
    parser.add_argument("--head", required=True)
    parser.add_argument("--tree", required=True)
    parser.add_argument("--run-id", required=True, type=int)
    args = parser.parse_args(argv)
    try:
        receipt = verify(args.repo, args.repo_id, args.head, args.tree, args.run_id,
                         os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN"))
    except Denied as exc:
        print("CI_EVIDENCE_DENIED: " + str(exc), file=sys.stderr)
        return 1
    print("CI_EXACT_HEAD_EXECUTION_VERIFIED " + json.dumps(receipt, sort_keys=True))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
