# Permission Denied Fixture

Use this scenario to create a managed settings path that exists but cannot be read by the scanner process.

The expected report is `fixtures/diagnostic/reports/permission-denied.json`.

Implementation tests should create permissions dynamically in a temporary directory rather than committing unreadable files to the repository.
