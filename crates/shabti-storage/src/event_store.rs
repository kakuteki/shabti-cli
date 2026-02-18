use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use shabti_core::error::{ShabtiError, ShabtiResult};
use shabti_core::Event;
use uuid::Uuid;

/// Stores events as JSON lines on disk with in-memory index.
pub struct EventStore {
    file: File,
    events: Vec<Event>,
    id_index: HashMap<Uuid, usize>,
    entry_index: HashMap<Uuid, usize>, // entry_id → event index
}

impl EventStore {
    /// Open or create an event store at the given path.
    /// Loads existing events on open.
    pub fn open<P: AsRef<Path>>(path: P) -> ShabtiResult<Self> {
        let path = path.as_ref();

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)
            .map_err(|e| ShabtiError::Io(e))?;

        let mut events = Vec::new();
        let mut id_index = HashMap::new();
        let mut entry_index = HashMap::new();

        // Read existing events
        let reader = BufReader::new(
            File::open(path).map_err(|e| ShabtiError::Io(e))?,
        );
        for line in reader.lines() {
            let line = line.map_err(|e| ShabtiError::Io(e))?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Event>(&line) {
                Ok(event) => {
                    let idx = events.len();
                    id_index.insert(event.id, idx);
                    for &eid in &event.entry_ids {
                        entry_index.insert(eid, idx);
                    }
                    events.push(event);
                }
                Err(_) => continue, // Skip malformed lines
            }
        }

        Ok(Self {
            file,
            events,
            id_index,
            entry_index,
        })
    }

    /// Add an event and persist it to disk.
    pub fn add(&mut self, event: Event) -> ShabtiResult<()> {
        let idx = self.events.len();
        self.id_index.insert(event.id, idx);
        for &eid in &event.entry_ids {
            self.entry_index.insert(eid, idx);
        }

        let mut line = serde_json::to_string(&event)
            .map_err(|e| ShabtiError::Serialization(e.to_string()))?;
        line.push('\n');
        self.file
            .write_all(line.as_bytes())
            .map_err(|e| ShabtiError::Io(e))?;

        self.events.push(event);
        Ok(())
    }

    /// Get an event by its ID.
    pub fn get(&self, id: Uuid) -> Option<&Event> {
        self.id_index.get(&id).map(|&idx| &self.events[idx])
    }

    /// Find the event containing a given entry ID.
    pub fn get_event_for_entry(&self, entry_id: Uuid) -> Option<&Event> {
        self.entry_index
            .get(&entry_id)
            .map(|&idx| &self.events[idx])
    }

    /// List all events.
    pub fn list(&self) -> &[Event] {
        &self.events
    }

    /// Number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
