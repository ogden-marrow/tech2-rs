# TIS2000 data formats

Two different data families live in a TIS2000 installation.

## Access / Jet

AppMan/COSIDS.MDB is a Microsoft Access database. It can be read read-only
with mdbtools or exported once to SQLite. The original file is never
modified.

Current Rust implementation:

~~~bash
sudo apt install mdbtools
cargo run -p tech2ctl -- tis export \
  --cosids "/path/to/Saab TIS2000 2.2008/cosids" \
  --out "$HOME/tech2-data"
~~~

This exported all 36 tables from the 2008 Saab build in testing. Set
TECH2_MDBTOOLS if mdbtools is not on PATH. A pure-Rust Jet reader is a later
option; the mdbtools path is the pragmatic read-only importer.

## SuperBase

spssaabdb, scasbdb, sdt2sbdb, and vcsaabdb are SuperBase databases:

- COMMAND.SC, CACHE.SC, alter.sc — control/schema files;
- rfile000 — the record data.

A native SuperBase reader is a reverse-engineering subproject. Two paths:

1. Write a read-only parser for the record format (clean long-term path).
2. One-time export with the original SuperBase engine into CSV/SQLite, then
   let the Rust app read SQLite. The shipped app has no Wine; the original
   data is untouched.

Current Rust implementation: tech2ctl tis probe reads the header and database
name from each rfile000 read-only. A full record parser is the next
reverse-engineering milestone; until then the original engine can be used for
a one-time export.

The card writer needs neither database. It needs the .bin image only.
