use shabti_core::MemoryEntry;
use shabti_storage::AppendLog;
use tempfile::TempDir;

fn make_entry(content: &str) -> MemoryEntry {
    MemoryEntry::new(
        content.to_string(),
        vec![0.1, 0.2, 0.3],
        "test-model".to_string(),
    )
}

#[test]
fn append_and_read_single_entry() {
    let dir = TempDir::new().unwrap();
    let mut log = AppendLog::open(dir.path().join("test.log")).unwrap();

    let entry = make_entry("hello world");
    let id = entry.id;
    log.append(&entry).unwrap();

    let restored = log.read(id).unwrap();
    assert_eq!(restored.id, id);
    assert_eq!(restored.content, "hello world");
    assert_eq!(restored.content_hash, entry.content_hash);
}

#[test]
fn append_multiple_and_read_each() {
    let dir = TempDir::new().unwrap();
    let mut log = AppendLog::open(dir.path().join("test.log")).unwrap();

    let entries: Vec<MemoryEntry> = (0..10).map(|i| make_entry(&format!("entry {i}"))).collect();
    for e in &entries {
        log.append(e).unwrap();
    }

    for e in &entries {
        let restored = log.read(e.id).unwrap();
        assert_eq!(restored.content, e.content);
    }
}

#[test]
fn read_nonexistent_returns_error() {
    let dir = TempDir::new().unwrap();
    let log = AppendLog::open(dir.path().join("test.log")).unwrap();

    let result = log.read(uuid::Uuid::new_v4());
    assert!(result.is_err());
}

#[test]
fn iter_returns_all_entries_in_order() {
    let dir = TempDir::new().unwrap();
    let mut log = AppendLog::open(dir.path().join("test.log")).unwrap();

    let entries: Vec<MemoryEntry> = (0..5).map(|i| make_entry(&format!("item {i}"))).collect();
    for e in &entries {
        log.append(e).unwrap();
    }

    let all: Vec<MemoryEntry> = log.iter().collect();
    assert_eq!(all.len(), 5);
    for (original, restored) in entries.iter().zip(all.iter()) {
        assert_eq!(original.id, restored.id);
        assert_eq!(original.content, restored.content);
    }
}

#[test]
fn len_tracks_entry_count() {
    let dir = TempDir::new().unwrap();
    let mut log = AppendLog::open(dir.path().join("test.log")).unwrap();
    assert_eq!(log.len(), 0);

    log.append(&make_entry("a")).unwrap();
    assert_eq!(log.len(), 1);

    log.append(&make_entry("b")).unwrap();
    assert_eq!(log.len(), 2);
}

#[test]
fn is_empty_on_new_log() {
    let dir = TempDir::new().unwrap();
    let log = AppendLog::open(dir.path().join("test.log")).unwrap();
    assert!(log.is_empty());
}

#[test]
fn reopen_preserves_entries() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.log");

    let entry = make_entry("persistent data");
    let id = entry.id;

    {
        let mut log = AppendLog::open(&path).unwrap();
        log.append(&entry).unwrap();
    }

    // Reopen
    let log = AppendLog::open(&path).unwrap();
    assert_eq!(log.len(), 1);
    let restored = log.read(id).unwrap();
    assert_eq!(restored.content, "persistent data");
}

#[test]
fn reopen_preserves_many_entries() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.log");

    let entries: Vec<MemoryEntry> = (0..100)
        .map(|i| make_entry(&format!("entry {i}")))
        .collect();

    {
        let mut log = AppendLog::open(&path).unwrap();
        for e in &entries {
            log.append(e).unwrap();
        }
    }

    let log = AppendLog::open(&path).unwrap();
    assert_eq!(log.len(), 100);
    for e in &entries {
        let restored = log.read(e.id).unwrap();
        assert_eq!(restored.content, e.content);
    }
}

#[test]
fn crc_integrity_check_on_reopen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.log");

    {
        let mut log = AppendLog::open(&path).unwrap();
        log.append(&make_entry("integrity test")).unwrap();
    }

    // Corrupt a byte in the middle of the file
    let mut data = std::fs::read(&path).unwrap();
    if data.len() > 20 {
        data[20] ^= 0xFF; // Flip bits
        std::fs::write(&path, &data).unwrap();
    }

    // Reopen should detect corruption and skip the entry
    let log = AppendLog::open(&path).unwrap();
    assert_eq!(log.len(), 0, "corrupted entry should be skipped on reopen");
}

#[test]
fn truncated_entry_is_skipped_on_reopen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.log");

    {
        let mut log = AppendLog::open(&path).unwrap();
        log.append(&make_entry("complete")).unwrap();
        log.append(&make_entry("will be truncated")).unwrap();
    }

    // Truncate file to cut off the second entry
    let data = std::fs::read(&path).unwrap();
    let truncated_len = data.len() - 10;
    std::fs::write(&path, &data[..truncated_len]).unwrap();

    let log = AppendLog::open(&path).unwrap();
    assert_eq!(log.len(), 1, "truncated entry should be skipped");
    let all: Vec<MemoryEntry> = log.iter().collect();
    assert_eq!(all[0].content, "complete");
}

#[test]
fn performance_1k_entries() {
    let dir = TempDir::new().unwrap();
    let mut log = AppendLog::open(dir.path().join("test.log")).unwrap();

    let start = std::time::Instant::now();
    for i in 0..1000 {
        log.append(&make_entry(&format!("perf entry {i}"))).unwrap();
    }
    let elapsed = start.elapsed();

    assert_eq!(log.len(), 1000);
    // Should complete in well under 5 seconds
    assert!(
        elapsed.as_secs() < 5,
        "1K inserts took {elapsed:?}, expected < 5s"
    );
}
