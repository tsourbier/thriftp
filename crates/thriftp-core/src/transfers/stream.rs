use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{CoreError, CoreResult};
use crate::protocol::TRANSFER_CHUNK_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamStats {
    pub bytes: u64,
    pub chunks: u64,
}

/// Copy a stream in fixed-size chunks while keeping memory use constant.
///
/// `on_chunk` receives the cumulative byte count for each written chunk and
/// returns `false` to stop at the next chunk boundary.
pub async fn copy_chunked<R, W, F>(
    reader: &mut R,
    writer: &mut W,
    chunk_size: usize,
    mut on_chunk: F,
) -> CoreResult<StreamStats>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(u64) -> bool,
{
    let chunk_size = chunk_size.max(1);
    let mut buffer = vec![0_u8; chunk_size];
    let mut bytes = 0_u64;
    let mut chunks = 0_u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| CoreError::Network(format!("read failed: {error}")))?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buffer[..read])
            .await
            .map_err(|error| CoreError::Network(format!("write failed: {error}")))?;
        bytes += read as u64;
        chunks += 1;
        if !on_chunk(bytes) {
            break;
        }
    }
    writer
        .flush()
        .await
        .map_err(|error| CoreError::Network(format!("flush failed: {error}")))?;
    Ok(StreamStats { bytes, chunks })
}

/// Default chunk size used by the transfer engine.
#[must_use]
pub fn default_chunk_size() -> usize {
    TRANSFER_CHUNK_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn copies_all_bytes_in_bounded_chunks() {
        let (mut writer, mut reader) = duplex(8);
        let producer = tokio::spawn(async move {
            writer.write_all(b"0123456789abcdef").await.unwrap();
            writer.shutdown().await.unwrap();
        });

        let mut output = Vec::new();
        let stats = copy_chunked(&mut reader, &mut output, 4, |_| true)
            .await
            .unwrap();
        producer.await.unwrap();

        assert_eq!(stats.bytes, 16);
        assert_eq!(stats.chunks, 4);
        assert_eq!(output, b"0123456789abcdef");
    }

    #[tokio::test]
    async fn stops_at_a_chunk_boundary_when_requested() {
        let (mut writer, mut reader) = duplex(8);
        let producer = tokio::spawn(async move {
            let _ = writer.write_all(b"0123456789abcdef").await;
        });

        let mut output = Vec::new();
        let stats = copy_chunked(&mut reader, &mut output, 4, |bytes| bytes < 8)
            .await
            .unwrap();

        assert_eq!(stats.bytes, 8);
        assert_eq!(output.len(), 8);
        producer.abort();
    }
}
