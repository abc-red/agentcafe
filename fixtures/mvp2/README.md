# MVP 2 Contract Fixtures

These fixtures freeze the MVP 2 write contract before implementation. They are expected results only; they must not contain raw secrets, prompt bodies, transcript bodies, shell output bodies, nonce plaintext, or snapshot payload content.

Fixture groups:

- `diff/*.json`: `config.diff` dry-run outputs.
- `apply/*.json`: `config.apply` result outputs.
- `snapshot/*.json`: snapshot manifest outputs.

Validate with:

```sh
node tests/integration/validate-mvp2-fixtures.mjs
```

MVP 1 behavior is unchanged: sidecar methods such as `config.diff`, `config.apply`, `backup.create`, and `backup.restore` must continue to return `feature_not_in_mvp` until MVP 2 implementation begins.
