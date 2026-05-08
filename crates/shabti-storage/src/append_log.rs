use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;

use shabti_core::entry::MemoryEntry;
use shabti_core::error::{ShabtiError, ShabtiResult};
use uuid::Uuid;

/// Record format (per entry):
///   [4 bytes] data_len (u32 LE)
///   [4 bytes] crc32   (u32 LE, over data bytes)
///   [data_len bytes] JSON-encoded MemoryEntry
const HEADER_SIZE: usize = 8; // 4 (len) + 4 (crc)

pub struct AppendLog {
    file: File,
    /// Maps entry UUID → byte offset in the file
    index: HashMap<Uuid, u64>,
    /// In-memory cache of all entries (for iter and read without seeking)
    entries: Vec<MemoryEntry>,
}

impl AppendLog {
    pub fn open<P: AsRef<Path>>(path: P) -> ShabtiResult<Self> {
        let path = path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut options = OpenOptions::new();
        options.create(true).read(true).append(true);

        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        let file = options.open(path)?;

        let mut log = Self {
            file,
            index: HashMap::new(),
            entries: Vec::new(),
        };

        log.rebuild_index(path)?;
        Ok(log)
    }

    pub fn append(&mut self, entry: &MemoryEntry) -> ShabtiResult<()> {
        let data =
            serde_json::to_vec(entry).map_err(|e| ShabtiError::Serialization(e.to_string()))?;
        let data_len = u32::try_from(data.len())
            .map_err(|_| ShabtiError::Storage("エントリのサイズが4GBを超えています".into()))?;
        let crc = crc32fast::hash(&data);

        let mut buf = Vec::with_capacity(HEADER_SIZE + data.len());
        buf.extend_from_slice(&data_len.to_le_bytes());
        buf.extend_from_slice(&crc.to_le_bytes());
        buf.extend_from_slice(&data);

        self.file.write_all(&buf)?;
        self.file.flush()?;

        let offset = self.entries.len() as u64;
        self.index.insert(entry.id, offset);
        self.entries.push(entry.clone());

        Ok(())
    }

    pub fn read(&self, id: Uuid) -> ShabtiResult<MemoryEntry> {
        let &idx = self
            .index
            .get(&id)
            .ok_or_else(|| ShabtiError::NotFound(format!("entry {id} not found in log")))?;
        Ok(self.entries[idx as usize].clone())
    }

    pub fn iter(&self) -> impl Iterator<Item = MemoryEntry> + '_ {
        self.entries.iter().cloned()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn rebuild_index<P: AsRef<Path>>(&mut self, path: P) -> ShabtiResult<()> {
        let file = File::open(path.as_ref())?;
        let file_len = file.metadata()?.len();
        if file_len == 0 {
            return Ok(());
        }

        let mut reader = BufReader::new(file);
        let mut pos: u64 = 0;

        loop {
            if pos + HEADER_SIZE as u64 > file_len {
                break;
            }

            // Read header
            let mut header = [0u8; HEADER_SIZE];
            match reader.read_exact(&mut header) {
                Ok(()) => {}
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(ShabtiError::Io(e)),
            }

            let data_len = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
            let stored_crc = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

            // Check if we have enough data
            if pos + HEADER_SIZE as u64 + data_len as u64 > file_len {
                // Truncated record — skip
                break;
            }

            // Read data
            let mut data = vec![0u8; data_len as usize];
            match reader.read_exact(&mut data) {
                Ok(()) => {}
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(ShabtiError::Io(e)),
            }

            // Verify CRC
            let computed_crc = crc32fast::hash(&data);
            if computed_crc != stored_crc {
                // Corrupted record — skip and stop (can't trust offsets after corruption)
                break;
            }

            // Deserialize
            match serde_json::from_slice::<MemoryEntry>(&data) {
                Ok(entry) => {
                    let idx = self.entries.len() as u64;
                    self.index.insert(entry.id, idx);
                    self.entries.push(entry);
                }
                Err(_) => {
                    // Corrupted JSON — skip and stop
                    break;
                }
            }

            pos += HEADER_SIZE as u64 + data_len as u64;
        }

        Ok(())
    }
}
