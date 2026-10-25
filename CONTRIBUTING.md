# Contributing

- Keep the repository code-only. Do not commit TIS2000/Card Writer binaries,
  card images, calibration data, Access/SuperBase files, or Wine.
- Do not commit decompiled output or copied proprietary code. Describe the
  behaviour and write a clean implementation.
- Do not add dongle-bypass or SPS/ECU-security-bypass code.
- Add tests for parsers and protocol state machines; use synthetic fixtures.
- Run:

~~~bash
scripts/check-no-proprietary.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
~~~

By contributing you agree that your code is licensed under the MIT license.
