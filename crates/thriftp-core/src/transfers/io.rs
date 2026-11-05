use std::io::SeekFrom;
use std::path::Path;

use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::error::{CoreError, CoreResult};
use crate::protocol::TRANSFER_CHUNK_SIZE;

pub struct LocalReader {
    file: File,
}

impl LocalReader {
    pub async fn open(path: &Path, offset: u64) -> CoreResult<Self> {
        let mut file = File::open(path).await.map_err(io_error)?;
        if offset > 0 {
            file.seek(SeekFrom::Start(offset)).await.map_err(io_error)?;
        }
        Ok(Self { file })
    }

    pub async fn read_chunk(&mut self, buffer: &mut [u8]) -> CoreResult<usize> {
        self.file.read(buffer).await.map_err(io_error)
    }
}

pub struct LocalWriter {
    file: File,
}

impl LocalWriter {
    pub async fn open(path: &Path, offset: u64, truncate: bool) -> CoreResult<Self> {
        let mut options = OpenOptions::new();
        options.create(true).write(true);
        if truncate {
            options.truncate(true);
        }
        let mut file = options.open(path).await.map_err(io_error)?;
        if offset > 0 {
            file.seek(SeekFrom::Start(offset)).await.map_err(io_error)?;
        }
        Ok(Self { file })
    }

    pub async fn write_chunk(&mut self, data: &[u8]) -> CoreResult<()> {
        self.file.write_all(data).await.map_err(io_error)
    }

    pub async fn finish(&mut self) -> CoreResult<()> {
        self.file.flush().await.map_err(io_error)
    }
}

/// Copy a local file to another local path.
pub async fn copy_local(
    source: &Path,
    destination: &Path,
    offset: u64,
    mut progress: impl FnMut(u64) -> bool,
) -> CoreResult<u64> {
    let mut reader = LocalReader::open(source, offset).await?;
    let mut writer = LocalWriter::open(destination, offset, offset == 0).await?;
    let mut buffer = vec![0_u8; TRANSFER_CHUNK_SIZE];
    let mut moved = 0_u64;
    loop {
        let read = reader.read_chunk(&mut buffer).await?;
        if read == 0 {
            break;
        }
        writer.write_chunk(&buffer[..read]).await?;
        moved += read as u64;
        if !progress(moved) {
            break;
        }
    }
    writer.finish().await?;
    Ok(moved)
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::Network(format!("local file error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn copies_local_files_with_an_offset() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.bin");
        let destination = directory.path().join("destination.bin");
        std::fs::write(&source, b"abcdef").unwrap();
        std::fs::write(&destination, b"XXXXXX").unwrap();

        let copied = copy_local(&source, &destination, 2, |_| true)
            .await
            .unwrap();
        assert_eq!(copied, 4);
        assert_eq!(std::fs::read(&destination).unwrap(), b"XXcdef");
    }
}
