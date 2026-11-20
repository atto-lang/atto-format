"""Offline fail-closed regressions; no network, source checkout, or CI upload."""
import unittest
from scripts.verify_private_ci_visibility import Denied, check_metadata, verify

SOURCE = "ores-truffle-oreslang/oreslang-format"
DESTINATION = "example-org/private-ci-carrier"


def metadata(name, number):
    return {"full_name": name, "id": number, "visibility": "private",
            "private": True, "archived": False, "disabled": False}


class PreflightTests(unittest.TestCase):
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

