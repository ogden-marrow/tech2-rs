# Card-writer protocol research

The native card writer is capture-driven. We do not guess the protocol.

## Capture options

- Linux socat pty proxy between COM1 and the real serial device, with hex
  logging.
- strace -f -e trace=read,write,ioctl -xx around the Windows writer under
  Wine during development.
- A logic analyzer on TX/RX/GND at 115200 for ground truth.
- A spare card and a known image.

## What to extract

- framing and baud;
- enter/erase/write/verify commands;
- block size and checksum;
- ACK/NAK, retry and timeout behaviour;
- card-size/version handshake.

## Deliverable

An original Rust implementation in tech2-card with a mock transport, a
capture transport, and a real serial transport. No proprietary code or data is
committed.
