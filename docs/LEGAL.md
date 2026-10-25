# Legal and clean-room policy

This project is a clean-room interoperability toolchain for hardware the user
owns. It is not legal advice.

## What the repository contains

- Original Rust code.
- Documentation written for this project.
- Tests that create synthetic files.

## What it must never contain

- TIS2000, Card Writer, Wine, or any other proprietary binary.
- Saab/GM .bin card images, SPS calibration data, or Access/SuperBase files.
- Decompiled code, copied strings, or reconstructed proprietary source.
- Dongle-bypass or SPS/ECU-security-bypass code.
- Secrets, tokens, or private keys.

## Data policy

Users supply their own legally acquired data. Importers open the original files
read-only, never write to them, and build a sidecar index under the user's
data directory. The original files remain the source of truth.

## Reverse engineering

Black-box protocol capture, file-format analysis, and interoperability
research are used to write new code. Decompilation of a user's own copy is a
research aid only; its output is not committed and is not copied into the
implementation. EULAs and copyright law vary by jurisdiction; consult a lawyer
for your situation.

## Deliberate exclusions

The project does not implement:

- the Tech 2 dongle check;
- SPS seed/key or ECU-flashing security;
- redistribution of factory calibration or card-image data.
