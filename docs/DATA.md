# TIS2000 data formats

Two different data families live in a TIS2000 installation.

## Access / Jet

AppMan/COSIDS.MDB is a Microsoft Access database. It can be read read-only
with mdbtools or exported once to SQLite. The original file is never
modified.

## SuperBase

spssaabdb, scasbdb, sdt2sbdb, and vcsaabdb are SuperBase databases:

- COMMAND.SC, CACHE.SC, alter.sc — control/schema files;
- rfile000 — the record data.

A native SuperBase reader is a reverse-engineering subproject. Two paths:

1. Write a read-only parser for the record format (clean long-term path).
2. One-time export with the original SuperBase engine into CSV/SQLite, then
   let the Rust app read SQLite. The shipped app has no Wine; the original
   data is untouched.

The card writer needs neither database. It needs the .bin image only.
