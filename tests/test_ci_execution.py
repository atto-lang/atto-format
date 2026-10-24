"""Offline safety tests for CI evidence receipts (no GitHub API calls)."""
import copy
import json
import os
import unittest
import urllib.request
from unittest import mock
from scripts.verify_ci_execution import Denied, NoRedirect, REQUIRED_STEPS, fetch_json, verify

REPO = "ores-truffle-oreslang/oreslang-format"
HEAD = "a" * 40
TREE = "b" * 40
RUN = 333333
ID = 1406328272

def fixture():
    jobs = []
    for name, commands in REQUIRED_STEPS.items():
        jobs.append({"name": name, "status": "completed", "conclusion": "success",
                     "runner_id": 101, "runner_name": "GitHub Actions 101", "run_attempt": 2,
                     "steps": [{"name": "Run " + command, "status": "completed",
                                "conclusion": "success"} for command in commands]})
    return {
        f"/repos/{REPO}": {"id": ID, "full_name": REPO, "private": True,
                            "visibility": "private", "archived": False, "disabled": False},
        f"/repos/{REPO}/git/commits/{HEAD}": {"sha": HEAD, "tree": {"sha": TREE}},
        f"/repos/{REPO}/actions/runs/{RUN}": {"head_sha": HEAD, "status": "completed",
                      "conclusion": "success", "event": "pull_request", "run_attempt": 2,
                      "repository": {"id": ID}},
        f"/repos/{REPO}/actions/runs/{RUN}/jobs?per_page=100": {"total_count": 2,
                                                                            "jobs": jobs},
    }

def run(data=None, **kwargs):
    data = fixture() if data is None else data
    return verify(REPO, ID, HEAD, TREE, RUN, "fake-token", fetch=lambda path, token: data[path], **kwargs)

class EvidenceTests(unittest.TestCase):
    def test_valid_exact_tree_with_real_steps(self):
        result = run()
        self.assertEqual(result["run_attempt"], 2)
        self.assertEqual(result["jobs"], ["msrv", "test"])

    def test_private_metadata_and_identity_are_required(self):
        path = f"/repos/{REPO}"
        for field, invalid in (("private", False), ("visibility", "public"),
                               ("archived", True), ("disabled", None),
                               ("id", True), ("id", ID+1), ("full_name", "other/repo")):
            with self.subTest(field=field, invalid=invalid):
                data = fixture(); data[path][field] = invalid
                with self.assertRaises(Denied): run(data)

    def test_mismatched_tree_and_commit_are_rejected(self):
        for change in (lambda d: d[f"/repos/{REPO}/git/commits/{HEAD}"]["tree"].update(sha="c"*40),
                       lambda d: d[f"/repos/{REPO}/git/commits/{HEAD}"].update(sha="c"*40),
                       lambda d: d[f"/repos/{REPO}/actions/runs/{RUN}"].update(head_sha="c"*40)):
            data = fixture(); change(data)
            with self.assertRaises(Denied): run(data)

    def test_run_failure_or_wrong_repository_is_rejected(self):
        path = f"/repos/{REPO}/actions/runs/{RUN}"
        for field, val in (("status", "queued"), ("conclusion", "failure"),
                           ("event", "unknown"), ("run_attempt", 0)):
            data=fixture();data[path][field]=val
            with self.subTest(field=field), self.assertRaises(Denied):run(data)
        data=fixture();data[path]["repository"]["id"]=123
        with self.assertRaises(Denied): run(data)

    def test_zero_step_no_runner_and_mixed_attempts_fail(self):
        path=f"/repos/{REPO}/actions/runs/{RUN}/jobs?per_page=100"
        for field,value in (("steps", []), ("runner_id", 0), ("runner_name", ""),
                            ("conclusion", "failure"), ("run_attempt", 1),
                            ("status", "queued")):
            data=fixture();data[path]["jobs"][0][field]=value
            with self.subTest(field=field),self.assertRaises(Denied):run(data)

    def test_missing_required_step_and_partial_jobs_fail(self):
        path=f"/repos/{REPO}/actions/runs/{RUN}/jobs?per_page=100"
        data=fixture();data[path]["jobs"][0]["steps"][0]["name"]="not run"
        with self.assertRaises(Denied):run(data)
        data=fixture();data[path]["total_count"]=100
        with self.assertRaises(Denied):run(data)
        data=fixture();data[path]["jobs"].pop();data[path]["total_count"]=1
        with self.assertRaises(Denied):run(data)

    def test_invalid_requests_fail_before_network(self):
        invalid=[("a/b", ID, HEAD, TREE, RUN, None), (REPO, True, HEAD, TREE, RUN, "fake"),
                 (REPO, ID, "bad", TREE, RUN, "fake"), (REPO, ID, HEAD, "bad", RUN, "fake")]
        for args in invalid:
            with self.subTest(args=args),self.assertRaises(Denied):
                verify(*args,fetch=lambda *a: self.fail("unexpected network"))

    def test_authenticated_http_disables_proxy_and_redirect(self):
        called=[]
        class Response:
            status=200
            def __init__(self,url):self.url=url
            def geturl(self):return self.url
            def read(self,n):return b'{"ok": true}'
            def __enter__(self):return self
            def __exit__(self,*args):return False
        class Opener:
            def open(self,req,timeout):
                called.append(req.get_header("Authorization"))
                return Response(req.full_url)
        def mock_opener(*handlers):
            self.assertTrue(any(isinstance(h,urllib.request.ProxyHandler) and h.proxies=={} for h in handlers))
            self.assertTrue(any(isinstance(h,NoRedirect) for h in handlers))
            return Opener()
        with mock.patch.dict(os.environ,{"HTTPS_PROXY":"http://untrusted.invalid:8080"}):
            with mock.patch("scripts.verify_ci_execution.urllib.request.build_opener",side_effect=mock_opener):
                self.assertEqual(fetch_json(f"/repos/{REPO}","fake-token"),{"ok":True})
        self.assertEqual(called,["Bearer fake-token"])

if __name__=="__main__":unittest.main()
