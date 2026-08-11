use std::io;

use forge_domain::{Conversation, ConversationId};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum serialized payload accepted by the daemon frame protocol.
const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

struct BoundedFrameWriter {
    bytes: Vec<u8>,
}

impl BoundedFrameWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(MAX_FRAME_BYTES),
        }
    }

    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }
}

impl io::Write for BoundedFrameWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() > MAX_FRAME_BYTES.saturating_sub(self.bytes.len()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "serialized frame exceeds maximum size",
            ));
        }
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    UpsertConversation {
        conversation: Conversation,
    },
    UpsertConversationRef {
        conversation: Conversation,
    },
    UpdateParentId {
        conversation_id: ConversationId,
        new_parent_id: Option<ConversationId>,
    },
    DeleteConversation {
        conversation_id: ConversationId,
    },
    OptimizeFts,
    RefreshFts,
    CheckpointWal,
    /// Health probe: returns daemon status without side effects.
    Ping,
}

/// Status returned by a [`Request::Ping`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Seconds the daemon has been running.
    pub uptime_secs: u64,
    /// Number of write requests currently queued (not yet flushed to disk).
    pub queue_depth: usize,
    /// Whether the database file/path is reachable (existence check for now).
    pub db_reachable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ack,
    Error {
        message: String,
    },
    /// Response to a [`Request::Ping`].
    Health(HealthStatus),
}

/// Async length-prefixed frame writer: writes a u32 length prefix plus JSON data.
pub async fn write_frame<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> io::Result<()> {
    let mut bounded = BoundedFrameWriter::new();
    {
        let mut serializer = serde_json::Serializer::new(&mut bounded);
        value
            .serialize(&mut serializer)
            .map_err(|e| io::Error::other(format!("JSON encoding error: {e}")))?;
    }
    let serialized = bounded.into_inner();
    let len = u32::try_from(serialized.len())
        .map_err(|_| io::Error::other("serialized frame exceeds u32 length limit"))?;
    writer.write_all(&len.to_le_bytes()).await?;
    writer.write_all(&serialized).await?;
    Ok(())
}

/// Async length-prefixed frame reader: reads a u32 length prefix and JSON payload.
/// data
pub async fn read_frame<R: AsyncRead + Unpin, T: for<'de> Deserialize<'de>>(
    reader: &mut R,
) -> io::Result<T> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > MAX_FRAME_BYTES {
        return Err(io::Error::other(format!("frame too large: {len} bytes")));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    serde_json::from_slice(&buf).map_err(|e| io::Error::other(format!("JSON decoding error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::{BoundedFrameWriter, Request, read_frame, write_frame};
    use std::io::Write;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn frame_payload_is_json_and_round_trips() {
        let (mut writer, mut reader) = tokio::io::duplex(1024);
        write_frame(&mut writer, &Request::Ping)
            .await
            .expect("write frame");

        let mut length = [0; 4];
        reader.read_exact(&mut length).await.expect("read length");
        let mut payload = vec![0; u32::from_le_bytes(length) as usize];
        reader.read_exact(&mut payload).await.expect("read payload");
        assert_eq!(payload, b"\"Ping\"", "payload was not JSON: {payload:?}");

        let (mut writer, mut reader) = tokio::io::duplex(1024);
        writer.write_all(&length).await.expect("write length");
        writer.write_all(&payload).await.expect("write payload");
        let actual: Request = read_frame(&mut reader).await.expect("round trip");
        assert!(matches!(actual, Request::Ping));
    }

    #[tokio::test]
    async fn read_frame_rejects_oversized_payload_before_allocation() {
        let (mut writer, mut reader) = tokio::io::duplex(1024);
        writer
            .write_all(&(8 * 1024 * 1024 + 1u32).to_le_bytes())
            .await
            .expect("write oversized length");
        let error = timeout(
            Duration::from_millis(100),
            read_frame::<_, Request>(&mut reader),
        )
        .await
        .expect("frame rejection should not wait for payload")
        .expect_err("oversized frame must fail closed");
        assert!(error.to_string().contains("frame too large"));
    }

    #[tokio::test]
    async fn write_frame_rejects_oversized_payload_before_writing() {
        let (mut writer, mut reader) = tokio::io::duplex(1024);
        let oversized = "x".repeat(super::MAX_FRAME_BYTES + 1);

        let error = write_frame(&mut writer, &oversized)
            .await
            .expect_err("oversized frame must fail closed");
        assert!(error.to_string().contains("serialized frame exceeds maximum size"));

        let mut prefix = [0u8; 4];
        assert!(timeout(Duration::from_millis(100), reader.read_exact(&mut prefix))
            .await
            .is_err(), "rejected frame must not write a prefix");
    }

    #[test]
    fn bounded_frame_writer_rejects_bytes_past_limit() {
        let mut writer = BoundedFrameWriter::new();
        let payload = vec![0u8; super::MAX_FRAME_BYTES];
        writer.write_all(&payload).expect("maximum frame fits");

        let error = writer
            .write_all(&[0u8])
            .expect_err("writer must reject bytes past maximum");
        assert!(error
            .to_string()
            .contains("serialized frame exceeds maximum size"));
    }
}
