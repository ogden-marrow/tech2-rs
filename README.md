# tech2-rs

A clean-room, open-source Rust toolchain for the Saab/GM Tech 2.

**Status: pre-alpha scaffold.** The workspace, image model, CLI skeleton, and
data-format probes are here. The native card-writer protocol and the data
importers are next.

## What this is

- A native **card writer** for Tech 2 PCMCIA images — no Wine, no Windows app.
- Read-only **data import** for the user's legally acquired TIS2000 files
  (Access .MDB and SuperBase rfile000 databases).
- A local **JSON-RPC daemon** (tech2d) and **CLI** (tech2ctl).
- A **GTK4/libadwaita** desktop app.
- A future **vehicle API** via J2534/CAN (separate from the Tech 2 serial link).

## What this is not

- It does **not** ship TIS2000, Card Writer, Wine, Saab/GM card images, or any
  proprietary database.
- It does **not** include a dongle bypass or an SPS/ECU-security bypass.
- It does **not** modify the user's data files. Importers open them read-only
  and build a sidecar index.

Bring your own legally acquired data. The repository is code and documentation
only.

## Workspace

| Crate | Role |
|---|---|
| tech2-core | image model, validation, versions, hashing |
| tech2-card | native card-writer protocol and transports |
| tech2-tis | read-only MDB/SuperBase import and indexing |
| tech2d | local daemon and JSON-RPC API |
| tech2ctl | command-line client |
| tech2-gui | GTK4/libadwaita app |

## Build

~~~bash
cargo test --workspace
cargo run -p tech2ctl -- inspect /path/to/Saab-148.bin
cargo run -p tech2ctl -- ports
scripts/check-no-proprietary.sh
~~~

## Legal and clean-room policy

The code is MIT. The data is not: TIS2000, Card Writer, Saab/GM card images and
calibration data keep their own terms. See [docs/LEGAL.md](docs/LEGAL.md) and
[CONTRIBUTING.md](CONTRIBUTING.md). Decompilation is used only for
understanding; the repository contains original clean-room code, not copied
decompiled output.
