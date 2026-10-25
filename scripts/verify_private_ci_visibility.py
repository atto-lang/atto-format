#!/usr/bin/env python3
"""Read-only preflight for a manually approved PRIVATE-to-PRIVATE CI transfer.

Necessary safety check, NOT authorization to copy source. Run before any Git
object, branch, or Actions artifact is written to an external repository.
"""
import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request

REPOSITORY = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*\Z", re.ASCII)
TOKEN = re.compile(r"[A-Za-z0-9._~+/-]+={0,16}\Z", re.ASCII)
MAX_METADATA = 131072


class Denied(Exception):
    pass


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, url):
        raise Denied("repository metadata redirect denied")


def validate_name(name):
    if not isinstance(name, str) or not REPOSITORY.fullmatch(name):
        raise Denied("invalid repository name")
    owner, repo = name.split("/", 1)
    if owner in (".", "..") or repo in (".", ".."):
        raise Denied("invalid repository name")


def check_metadata(metadata, name, expected_id):
    if not isinstance(metadata, dict):
        raise Denied("invalid repository metadata")
    if (metadata.get("private") is not True or
            metadata.get("visibility") != "private" or
            metadata.get("archived") is not False or
            metadata.get("disabled") is not False):
        raise Denied("repository not verified active/private")
    if (not isinstance(metadata.get("full_name"), str) or
            metadata["full_name"].casefold() != name.casefold() or
            type(metadata.get("id")) is not int or metadata["id"] != expected_id):
        raise Denied("repository identity mismatch")


def fetch_metadata(name, token):
    if not isinstance(token, str) or not (1 <= len(token) <= 4096) or not TOKEN.fullmatch(token):
        raise Denied("authenticated repository metadata required")
    request = urllib.request.Request(
        "https://api.github.com/repos/" + name,
        headers={"Accept": "application/vnd.github+json",
                 "Authorization": "Bearer " + token,
                 "X-GitHub-Api-Version": "2022-11-28",
                 "User-Agent": "oreslang-format-private-ci-preflight"},
        method="GET")
    try:
        with urllib.request.build_opener(NoRedirect()).open(request, timeout=8) as response:
            if response.status != 200 or response.geturl() != request.full_url:
                raise Denied("repository metadata response invalid")
            raw = response.read(MAX_METADATA + 1)
    except (urllib.error.URLError, TimeoutError, OSError, ValueError):
        raise Denied("repository metadata unavailable") from None
    if len(raw) > MAX_METADATA:
        raise Denied("repository metadata too large")
    try:
        return json.loads(raw.decode("utf-8"))
    except (UnicodeError, ValueError):
        raise Denied("repository metadata invalid") from None


def verify(source, destination, source_id, destination_id, token, fetch=fetch_metadata):
    validate_name(source)
    validate_name(destination)
    if source.casefold() == destination.casefold():
        raise Denied("source and destination must differ")
    if (type(source_id) is not int or type(destination_id) is not int
            or source_id <= 0 or destination_id <= 0 or source_id == destination_id):
        raise Denied("immutable repository IDs required")
    if not isinstance(token, str) or not (1 <= len(token) <= 4096) or not TOKEN.fullmatch(token):
        raise Denied("authenticated repository metadata required")
    for name, expected_id in ((source, source_id), (destination, destination_id)):
        check_metadata(fetch(name, token), name, expected_id)
    return True


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--source-id", required=True, type=int)
    parser.add_argument("--destination", required=True)
    parser.add_argument("--destination-id", required=True, type=int)
    args = parser.parse_args(argv)
    try:
        verify(args.source, args.destination, args.source_id, args.destination_id,
               os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN"))
    except Denied as exc:
        print("PRIVATE_CI_PREFLIGHT_DENIED: " + str(exc), file=sys.stderr)
        return 1
    print("PRIVATE_CI_VISIBILITY_OK (authorization and incident clearance still required)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

