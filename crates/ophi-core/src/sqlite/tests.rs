use super::*;
use crate::{MAX_RECORD_BYTES, Reference};
use std::sync::{Arc, Barrier};

fn repo() -> SqliteRepository {
    SqliteRepository::open(":memory:").unwrap()
}
fn case(repo: &mut SqliteRepository) -> Case {
    repo.create_case(NewCase {
        title: "Investigate timeout".into(),
    })
    .unwrap()
}
fn request(case_id: &str) -> NewEntry {
    NewEntry {
        case_id: case_id.into(),
        kind: "failure".into(),
        content: "Attempt failed: pool exhausted".into(),
        references: vec![],
        author: "human".into(),
    }
}
fn reference(target: &Entry) -> Reference {
    Reference {
        target_id: target.id.clone(),
        relation: "supersedes".into(),
    }
}

#[test]
fn records_survive_reopen_with_exact_content_and_references() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.db");
    let mut db = SqliteRepository::open(&path).unwrap();
    let case = case(&mut db);
    let first = db.append(request(&case.id)).unwrap();
    let mut input = request(&case.id);
    input.content = "修订：增加连接池容量后恢复".into();
    input.references.push(reference(&first));
    let second = db.append(input).unwrap();
    drop(db);
    let db = SqliteRepository::open(path).unwrap();
    assert_eq!(db.get_case(&case.id).unwrap(), Some(case.clone()));
    assert_eq!(db.get(&first.id).unwrap(), Some(first.clone()));
    assert_eq!(db.get(&second.id).unwrap(), Some(second.clone()));
    assert_eq!(
        db.history(&case.id, None, 100).unwrap().entries,
        vec![first, second]
    );
}

#[test]
fn duplicate_id_update_and_delete_cannot_replace_original() {
    let mut db = repo();
    let case = case(&mut db);
    let entry = db.append(request(&case.id)).unwrap();
    assert!(
        db.connection
            .execute(
                "INSERT INTO entries(id,case_id,record) VALUES (?1,?2,'{}')",
                params![entry.id, case.id]
            )
            .is_err()
    );
    assert!(
        db.connection
            .execute(
                "INSERT OR REPLACE INTO entries(id,case_id,record) VALUES (?1,?2,'{}')",
                params![entry.id, case.id],
            )
            .is_err()
    );
    for table in ["cases", "entries"] {
        assert!(
            db.connection
                .execute(&format!("UPDATE {table} SET record='{{}}'"), [])
                .is_err()
        );
        assert!(
            db.connection
                .execute(&format!("DELETE FROM {table}"), [])
                .is_err()
        );
    }
    assert_eq!(db.get(&entry.id).unwrap(), Some(entry));
}

#[test]
fn missing_case_missing_target_and_cross_case_references_are_rejected() {
    let mut db = repo();
    let first_case = case(&mut db);
    let second_case = case(&mut db);
    let target = db.append(request(&first_case.id)).unwrap();
    assert!(matches!(
        db.append(request(&uuid::Uuid::new_v4().to_string())),
        Err(Error::NotFound { .. })
    ));
    let mut input = request(&second_case.id);
    input.references.push(reference(&target));
    assert!(matches!(db.append(input), Err(Error::InvalidReference(_))));
    let mut input = request(&second_case.id);
    input.references.push(Reference {
        target_id: uuid::Uuid::new_v4().to_string(),
        relation: "supports".into(),
    });
    assert!(matches!(db.append(input), Err(Error::InvalidReference(_))));
    assert!(
        db.history(&second_case.id, None, 10)
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn storage_failure_mid_write_rolls_back_entry_and_all_references() {
    let mut db = repo();
    let case = case(&mut db);
    let a = db.append(request(&case.id)).unwrap();
    let b = db.append(request(&case.id)).unwrap();
    db.connection.execute_batch("CREATE TRIGGER simulate_disk_failure BEFORE INSERT ON entry_references WHEN NEW.position=1 BEGIN SELECT RAISE(ABORT, 'simulated write failure'); END;").unwrap();
    let mut input = request(&case.id);
    input.references = vec![reference(&a), reference(&b)];
    assert!(matches!(db.append(input), Err(Error::Storage(_))));
    assert_eq!(db.history(&case.id, None, 10).unwrap().entries, vec![a, b]);
    let count: i64 = db
        .connection
        .query_row("SELECT count(*) FROM entry_references", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    db.connection
        .execute_batch("DROP TRIGGER simulate_disk_failure")
        .unwrap();
    assert!(db.append(request(&case.id)).is_ok());
}

#[test]
fn references_cannot_be_updated_or_deleted() {
    let mut db = repo();
    let case = case(&mut db);
    let target = db.append(request(&case.id)).unwrap();
    let mut input = request(&case.id);
    input.references.push(reference(&target));
    db.append(input).unwrap();
    assert!(
        db.connection
            .execute("UPDATE entry_references SET relation='verified'", [])
            .is_err()
    );
    assert!(
        db.connection
            .execute("DELETE FROM entry_references", [])
            .is_err()
    );
}

#[test]
fn pagination_is_by_append_sequence_even_with_identical_timestamps() {
    let mut db = repo();
    let case = case(&mut db);
    let mut expected = Vec::new();
    // Fixed timestamps deliberately make chronological sorting ambiguous.
    for _ in 0..5 {
        let mut entry = Entry::create(request(&case.id)).unwrap();
        entry.created_at = "2026-01-01T00:00:00Z".into();
        db.connection
            .execute(
                "INSERT INTO entries(id,case_id,record) VALUES (?1,?2,?3)",
                params![entry.id, case.id, encode(&entry).unwrap()],
            )
            .unwrap();
        expected.push(entry);
    }
    let mut cursor = None;
    let mut actual = Vec::new();
    loop {
        let page = db.history(&case.id, cursor.as_deref(), 2).unwrap();
        actual.extend(page.entries);
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(actual, expected);
    assert!(db.history(&case.id, None, 0).is_err());
    assert!(db.history(&case.id, None, 1001).is_err());
    for cursor in ["0", "-1", "abc", "01", "9223372036854775808"] {
        assert!(db.history(&case.id, Some(cursor), 2).is_err());
    }
}

#[test]
fn invalid_input_does_not_prevent_later_writes() {
    let mut db = repo();
    let case = case(&mut db);
    for content in [
        " ".to_string(),
        "x".repeat(MAX_RECORD_BYTES),
        "\u{0000}".repeat(MAX_RECORD_BYTES / 2),
    ] {
        let mut input = request(&case.id);
        input.content = content;
        assert!(matches!(db.append(input), Err(Error::Validation(_))));
    }
    let target = db.append(request(&case.id)).unwrap();
    let mut input = request(&case.id);
    input.references = vec![reference(&target), reference(&target)];
    assert!(db.append(input).is_err());
    assert!(db.get("../../etc/passwd").is_err());
    assert!(db.append(request(&case.id)).is_ok());
    assert_eq!(db.history(&case.id, None, 10).unwrap().entries.len(), 2);
}

#[test]
fn submitted_proof_is_preserved_without_inferred_verification() {
    let mut db = repo();
    let case = case(&mut db);
    let mut input = request(&case.id);
    input.kind = "proof_submitted".into();
    let entry = db.append(input).unwrap();
    let json = serde_json::to_value(db.get(&entry.id).unwrap().unwrap()).unwrap();
    assert_eq!(json["kind"], "proof_submitted");
    assert!(json.get("status").is_none());
}

#[test]
fn unknown_json_fields_and_caller_assigned_metadata_are_rejected() {
    assert!(serde_json::from_str::<NewCase>(r#"{"title":"test","id":"caller-id"}"#).is_err());
    let mut input = serde_json::to_value(request(&uuid::Uuid::new_v4().to_string())).unwrap();
    for field in ["id", "created_at", "schema_version", "status"] {
        input[field] = serde_json::json!("unexpected");
        assert!(serde_json::from_value::<NewEntry>(input.clone()).is_err());
        input.as_object_mut().unwrap().remove(field);
    }
}

#[test]
fn unsupported_schema_is_not_silently_downgraded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    let conn = Connection::open(&path).unwrap();
    conn.pragma_update(None, "user_version", 99).unwrap();
    drop(conn);
    assert!(matches!(
        SqliteRepository::open(&path),
        Err(Error::UnsupportedSchema(99))
    ));
    let conn = Connection::open(path).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        99
    );
}

#[test]
fn concurrent_connections_preserve_all_appends_and_references() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shared.db");
    let mut db = SqliteRepository::open(&path).unwrap();
    let case = case(&mut db);
    let target = db.append(request(&case.id)).unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let path = path.clone();
            let case_id = case.id.clone();
            let target = target.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let mut db = SqliteRepository::open(path).unwrap();
                for _ in 0..10 {
                    let mut input = request(&case_id);
                    input.references.push(reference(&target));
                    db.append(input).unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(db.history(&case.id, None, 100).unwrap().entries.len(), 41);
}
