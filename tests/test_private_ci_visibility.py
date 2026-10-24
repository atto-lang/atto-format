"""Offline fail-closed regressions; no network, source checkout, or CI upload."""
import unittest
import json
import os
import urllib.request
from unittest import mock
from scripts.verify_private_ci_visibility import Denied, NoRedirect, check_metadata, fetch_metadata, verify

SOURCE = "ores-truffle-oreslang/oreslang-format"
DESTINATION = "example-org/private-ci-carrier"


def metadata(name, number):
    return {"full_name": name, "id": number, "visibility": "private",
            "private": True, "archived": False, "disabled": False}


class PreflightTests(unittest.TestCase):
    def test_authenticated_metadata_fetch_disables_ambient_proxy(self):
        # A CI host may set HTTPS_PROXY. Never send the scoped read token
        # through an inherited proxy just to check repository visibility.
        expected = metadata(DESTINATION, 202)
        called = []

        class Response:
            status = 200
            def __init__(self, url):
                self.url = url
            def geturl(self):
                return self.url
            def read(self, limit):
                self.limit = limit
                return json.dumps(expected).encode("utf-8")
            def __enter__(self):
                return self
            def __exit__(self, exc_type, exc_value, traceback):
                return False

        class Opener:
            def open(self, request, timeout):
                called.append((request, timeout))
                return Response(request.full_url)

        def build_opener(*handlers):
            self.assertTrue(any(
                isinstance(h, urllib.request.ProxyHandler) and h.proxies == {}
                for h in handlers), handlers)
            self.assertTrue(any(isinstance(h, NoRedirect) for h in handlers))
            return Opener()

        with mock.patch.dict(os.environ, {"HTTPS_PROXY": "http://untrusted.invalid:8123"}):
            with mock.patch("scripts.verify_private_ci_visibility.urllib.request.build_opener",
                            side_effect=build_opener):
                self.assertEqual(fetch_metadata(DESTINATION, "fake-token"), expected)

        self.assertEqual(len(called), 1)
        self.assertEqual(called[0][0].get_header("Authorization"), "Bearer fake-token")
        self.assertEqual(called[0][1], 8)

    def test_approved_metadata_only(self):
        calls = []
        def fetch(name, token):
            calls.append(name)
            return metadata(name, 101 if name == SOURCE else 202)
        self.assertTrue(verify(SOURCE, DESTINATION, 101, 202, "fake-token", fetch))
        self.assertEqual(calls, [SOURCE, DESTINATION])

    def test_public_destination_denied(self):
        def fetch(name, token):
            value = metadata(name, 101 if name == SOURCE else 202)
            if name == DESTINATION:
                value.update(private=False, visibility="public")
            return value
        with self.assertRaisesRegex(Denied, "private"):
            verify(SOURCE, DESTINATION, 101, 202, "fake-token", fetch)

    def test_public_source_denied(self):
        def fetch(name, token):
            value = metadata(name, 101 if name == SOURCE else 202)
            if name == SOURCE:
                value["visibility"] = "public"
            return value
        with self.assertRaises(Denied):
            verify(SOURCE, DESTINATION, 101, 202, "fake-token", fetch)

    def test_identity_swap_denied(self):
        def fetch(name, token):
            return metadata(name, 202 if name == SOURCE else 101)
        with self.assertRaisesRegex(Denied, "identity"):
            verify(SOURCE, DESTINATION, 101, 202, "fake-token", fetch)

    def test_archived_disabled_unknown_denied(self):
        for field, invalid in [("archived", True), ("archived", None),
                               ("disabled", True), ("disabled", None)]:
            with self.subTest(field=field, invalid=invalid):
                data = metadata(DESTINATION, 202)
                data[field] = invalid
                with self.assertRaises(Denied):
                    check_metadata(data, DESTINATION, 202)

    def test_bool_id_rejected(self):
        data = metadata(DESTINATION, True)
        with self.assertRaisesRegex(Denied, "identity"):
            check_metadata(data, DESTINATION, 1)

    def test_renamed_repository_denied(self):
        with self.assertRaisesRegex(Denied, "identity"):
            check_metadata(metadata("other-org/private-ci-carrier", 202), DESTINATION, 202)

    def test_missing_or_bad_token_denies_before_fetch(self):
        for value in [None, "", "bad\r\ntoken", "bad token"]:
            with self.subTest(value=repr(value)):
                def forbidden_fetch(name, token):
                    self.fail("must not fetch without valid token")
                with self.assertRaises(Denied):
                    verify(SOURCE, DESTINATION, 101, 202, value, forbidden_fetch)

    def test_invalid_names_denied_before_fetch(self):
        for value in ["../repo", "name", "/repo", "a/.","a/../b","a/b?x=1"]:
            with self.subTest(value=value):
                with self.assertRaises(Denied):
                    verify(SOURCE, value, 101, 202, "fake-token",
                           lambda name, token: self.fail("unexpected fetch"))

    def test_identical_names_and_ids_denied(self):
        for destination, source_id, destination_id in [
            (SOURCE, 101, 202), (DESTINATION, 101, 101),
            (DESTINATION, False, 202), (DESTINATION, 101, 0),
            (DESTINATION, "101", 202)]:
            with self.subTest(destination=destination, source_id=source_id,
                              destination_id=destination_id):
                with self.assertRaises(Denied):
                    verify(SOURCE, destination, source_id, destination_id, "fake-token",
                           lambda name, token: self.fail("unexpected fetch"))


if __name__ == "__main__":
    unittest.main()

