# tech2d JSON-RPC API

tech2d is a local daemon. It owns the Unix socket, serial discovery, image
inspection, and read-only TIS data queries. The CLI and GUI are clients.

## Socket

Default resolution:

1. TECH2D_SOCKET, if set;
2. XDG_RUNTIME_DIR/tech2d.sock, if XDG_RUNTIME_DIR is set;
3. /tmp/tech2d-UID.sock.

Start with an explicit socket:

~~~bash
tech2d --socket /tmp/tech2d.sock
~~~

The socket is created with mode 0600.

## Wire format

Newline-delimited JSON-RPC 2.0. One JSON object per line. Multiple requests
may be sent on one connection.

~~~json
{"jsonrpc":"2.0","id":1,"method":"status","params":{}}
~~~

Response:

~~~json
{"jsonrpc":"2.0","id":1,"result":{"name":"tech2d"}}
~~~

Errors use standard JSON-RPC codes:

| Code | Meaning |
|---|---|
| -32700 | parse error |
| -32600 | invalid request |
| -32601 | method not found |
| -32602 | invalid params |
| -32000 | server error / not implemented |

## Methods

| Method | Params | Result |
|---|---|---|
| status | none | daemon version, socket, capabilities |
| ports.list | none | candidate serial devices |
| images.inspect | path, optional car | image size/version/coverage/warnings |
| tis.tables | cosids | table names from COSIDS.MDB |
| tis.probe | cosids | SuperBase rfile header probes |
| card.capabilities | none | native writer availability |
| card.write | port, image, options | reserved; currently an error |

### examples

~~~bash
tech2ctl rpc status --socket /tmp/tech2d.sock
tech2ctl rpc ports.list --socket /tmp/tech2d.sock
tech2ctl rpc images.inspect \
  '{"path":"/path/to/Saab 148.bin"}' --socket /tmp/tech2d.sock
tech2ctl rpc tis.tables \
  '{"cosids":"/path/to/cosids"}' --socket /tmp/tech2d.sock
tech2ctl rpc tis.probe \
  '{"cosids":"/path/to/cosids"}' --socket /tmp/tech2d.sock
~~~

## Data access

tis.tables uses the read-only mdbtools export path. Install mdbtools or point
TECH2_MDBTOOLS at its bin directory. No method writes to TIS2000 or SuperBase
files.

## Security

- Local Unix socket only; no TCP listener.
- Socket mode 0600.
- No shell execution of user strings; mdbtools is invoked with explicit args.
- Data paths are canonicalized, and export refuses to write inside the source
  tree.

## Roadmap

- job queue with progress and cancellation;
- card.write native implementation after serial capture;
- tis.export with a sidecar SQLite index;
- GUI as a daemon client.
