# Architecture

~~~
tech2-gui / tech2ctl
        │  JSON-RPC over a Unix socket
        ▼
      tech2d ──────────────┐
        │                  │
        ├── tech2-card     │ native serial protocol
        ├── tech2-tis      │ read-only data import + SQLite index
        └── tech2-core     │ image model, validation, hashing
~~~

## Design rules

- The serial port is owned by tech2d, not by the GUI.
- The GUI and CLI are clients of the same API.
- Data import is read-only; an index lives under the user's data directory.
- The card writer is transport-agnostic: a real serial transport, a capture
  proxy, and a mock transport implement the same trait.
- No proprietary code or data enters the repository.

## API shape (draft)

~~~
ports.list
images.inspect
images.list
card.write
card.verify
card.backup
job.status
job.cancel
tis.tables
tis.query
~~~
