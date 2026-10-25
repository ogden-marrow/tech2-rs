# Security

This project talks to embedded hardware and reads user-owned databases.

- Do not report vulnerabilities with real secrets, VINs, or proprietary data.
- Never commit tokens, private keys, or database dumps.
- Serial and file parsers must fail closed and must not write to source data.
- Report issues privately via the repository's security contact.
