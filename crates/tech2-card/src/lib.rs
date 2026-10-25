//! Native card-writer protocol scaffold.

use std::io;

/// Errors from the card writer.
#[derive(Debug)]
pub enum CardError {
    Io(io::Error),
    Protocol(String),
    Unimplemented,
}

impl std::fmt::Display for CardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Protocol(msg) => write!(f, "protocol error: {msg}"),
            Self::Unimplemented => write!(f, "native card-writer protocol is not implemented yet"),
        }
    }
}

impl std::error::Error for CardError {}

impl From<io::Error> for CardError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

/// A byte transport to the Tech 2.
pub trait CardTransport {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

/// High-level card writer. The protocol implementation lands after capture.
pub struct CardWriter<T: CardTransport> {
    transport: T,
}

impl<T: CardTransport> CardWriter<T> {
    #[allow(clippy::new_without_default)]
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn transport(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn write_image(&mut self, _image: &[u8]) -> Result<(), CardError> {
        Err(CardError::Unimplemented)
    }
}

/// Mock transport for tests and protocol development.
#[derive(Debug, Default)]
pub struct MockTransport {
    pub written: Vec<u8>,
    pub to_read: Vec<u8>,
}

impl CardTransport for MockTransport {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.written.extend_from_slice(buf);
        Ok(())
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        if self.to_read.len() < buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "mock data exhausted",
            ));
        }
        buf.copy_from_slice(&self.to_read[..buf.len()]);
        self.to_read.drain(..buf.len());
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_transport_records_bytes() {
        let mut mock = MockTransport::default();
        mock.write_all(b"hello").unwrap();
        mock.flush().unwrap();
        assert_eq!(mock.written, b"hello");
    }

    #[test]
    fn writer_is_explicitly_unimplemented_for_now() {
        let mut writer = CardWriter::new(MockTransport::default());
        assert!(matches!(
            writer.write_image(&[0u8; 4]),
            Err(CardError::Unimplemented)
        ));
    }
}
