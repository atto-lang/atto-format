# Private-source CI preflight — formatter

The formatter source repository is private. A prior test carrier was public; see
[security issue #70](https://github.com/ores-truffle-oreslang/oreslang-format/issues/70).
Never reuse a public carrier for private code or assume that resetting a branch
removes historical Git objects or Actions artifacts.

Before any *separately approved* private CI source transfer, validate BOTH the
private source and private destination identities with an authenticated GitHub
API request. Use a token authorized only to read repository metadata.

```sh
GH_TOKEN=<read-only-token> python3 scripts/verify_private_ci_visibility.py \
  --source ores-truffle-oreslang/oreslang-format \
  --source-id <reviewed-source-numeric-repository-id> \
  --destination <reviewed-private-owner/repo> \
  --destination-id <reviewed-destination-numeric-repository-id>
```

The metadata request intentionally disables ambient HTTP(S) proxy configuration
rather than sending a read-only GitHub bearer credential to an unreviewed proxy.
A network that requires such a proxy fails closed; do not bypass this by
embedding a token in a URL or swapping to unauthenticated repository metadata.

The tool is read-only and prints only a generic success or denial message.
It does NOT mirror source, approve a destination, prove independent human
review, establish an immutable source commit/tree, or validate later
visibility changes. Success is a **necessary but insufficient** gate.

Requirements independent of this preflight:

- Incident #70 must receive owner/security disposition, including historical
  Git-object and Actions retention, access assessment, and GitHub Support purge
  follow-up. A safe future carrier does not erase a past exposure.
- Authorize the exact source commit and root Git tree, exact destination numeric
  ID, and security reviewer through an independently enforced protected policy.
- Recheck source and destination identity/privacy before **each write** in the
  eventual source publisher; a one-time preflight cannot close TOCTOU races.
- Do not enable private-source mirroring until the verified publisher
  implements all those controls and a genuinely independent reviewer approves.

Offline negative tests (no GitHub credentials or network needed):

```sh
python3 -m unittest discover -s tests -p test_private_ci_visibility.py -v
```

Do not treat a GitHub Actions workflow that fails before runner allocation as
executable test evidence. Keep the corresponding PR in draft until the exact
head has been tested on an approved runner.

