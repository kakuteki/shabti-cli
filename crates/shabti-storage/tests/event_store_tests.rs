use shabti_core::Event;
use shabti_storage::EventStore;
use tempfile::TempDir;
use uuid::Uuid;

fn make_event(n: usize) -> Event {
    let entry_ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
    Event::new(entry_ids, 1000, 2000)
}

#[test]
fn create_and_get_event() {
    let dir = TempDir::new().unwrap();
    let mut store = EventStore::open(dir.path().join("events.jsonl")).unwrap();

    let event = make_event(3);
    let id = event.id;
    store.add(event).unwrap();

    let retrieved = store.get(id).unwrap();
    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.entry_ids.len(), 3);
    assert_eq!(retrieved.start_time, 1000);
    assert_eq!(retrieved.end_time, 2000);
}

#[test]
fn get_nonexistent_returns_none() {
    let dir = TempDir::new().unwrap();
    let store = EventStore::open(dir.path().join("events.jsonl")).unwrap();

    assert!(store.get(Uuid::new_v4()).is_none());
}

#[test]
fn get_event_for_entry() {
    let dir = TempDir::new().unwrap();
    let mut store = EventStore::open(dir.path().join("events.jsonl")).unwrap();

    let entry_id = Uuid::new_v4();
    let mut event = make_event(2);
    event.entry_ids.push(entry_id);
    let event_id = event.id;
    store.add(event).unwrap();

    let found = store.get_event_for_entry(entry_id).unwrap();
    assert_eq!(found.id, event_id);
}

#[test]
fn get_event_for_entry_not_found() {
    let dir = TempDir::new().unwrap();
    let store = EventStore::open(dir.path().join("events.jsonl")).unwrap();

    assert!(store.get_event_for_entry(Uuid::new_v4()).is_none());
}

#[test]
fn list_events() {
    let dir = TempDir::new().unwrap();
    let mut store = EventStore::open(dir.path().join("events.jsonl")).unwrap();

    store.add(make_event(2)).unwrap();
    store.add(make_event(3)).unwrap();
    store.add(make_event(1)).unwrap();

    let all = store.list();
    assert_eq!(all.len(), 3);
}

#[test]
fn len_and_is_empty() {
    let dir = TempDir::new().unwrap();
    let mut store = EventStore::open(dir.path().join("events.jsonl")).unwrap();

    assert!(store.is_empty());
    assert_eq!(store.len(), 0);

    store.add(make_event(1)).unwrap();
    assert!(!store.is_empty());
    assert_eq!(store.len(), 1);
}

#[test]
fn persistence_across_reopen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("events.jsonl");

    let event = make_event(5);
    let id = event.id;

    {
        let mut store = EventStore::open(&path).unwrap();
        store.add(event).unwrap();
    }

    // Reopen
    let store = EventStore::open(&path).unwrap();
    assert_eq!(store.len(), 1);
    let retrieved = store.get(id).unwrap();
    assert_eq!(retrieved.entry_ids.len(), 5);
}

#[test]
fn persistence_many_events() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("events.jsonl");

    let events: Vec<Event> = (0..50).map(|i| make_event(i % 5 + 1)).collect();
    let ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();

    {
        let mut store = EventStore::open(&path).unwrap();
        for e in events {
            store.add(e).unwrap();
        }
    }

    let store = EventStore::open(&path).unwrap();
    assert_eq!(store.len(), 50);
    for id in ids {
        assert!(store.get(id).is_some());
    }
}

#[test]
fn event_summary_embedding_optional() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("events.jsonl");

    let mut event = make_event(2);
    event.summary_embedding = Some(vec![0.1, 0.2, 0.3]);
    let id = event.id;

    {
        let mut store = EventStore::open(&path).unwrap();
        store.add(event).unwrap();
    }

    let store = EventStore::open(&path).unwrap();
    let retrieved = store.get(id).unwrap();
    assert_eq!(
        retrieved.summary_embedding.as_ref().unwrap(),
        &vec![0.1, 0.2, 0.3]
    );
}
