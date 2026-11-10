use crate::{
    Case, Entry, Error, HistoryPage, NewCase, NewEntry, Repository,
    model::{encode, valid_id},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{path::Path, time::Duration};

pub struct SqliteRepository {
    connection: Connection,
}

impl SqliteRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "recursive_triggers", true)?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        match version {
            0 => {
                tx.execute_batch(SCHEMA)?;
                tx.pragma_update(None, "user_version", 1)?;
            }
            1 => {}
            other => return Err(Error::UnsupportedSchema(other)),
        }
        tx.commit()?;
        Ok(Self { connection })
    }
}

impl Repository for SqliteRepository {
    fn create_case(&mut self, input: NewCase) -> Result<Case, Error> {
        let case = Case::create(input)?;
        let json = encode(&case)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO cases (id, record) VALUES (?1, ?2)",
            params![case.id, json],
        )?;
        tx.commit()?;
        Ok(case)
    }

    fn get_case(&self, id: &str) -> Result<Option<Case>, Error> {
        valid_id(id)?;
        let json: Option<String> = self
            .connection
            .query_row("SELECT record FROM cases WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        json.map(|j| serde_json::from_str(&j).map_err(Error::from))
            .transpose()
    }

    fn append(&mut self, input: NewEntry) -> Result<Entry, Error> {
        let entry = Entry::create(input)?;
        let json = encode(&entry)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM cases WHERE id=?1)",
            [&entry.case_id],
            |r| r.get(0),
        )?;
        if !exists {
            return Err(Error::NotFound {
                entity: "case",
                id: entry.case_id,
            });
        }
        for reference in &entry.references {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM entries WHERE id=?1 AND case_id=?2)",
                params![reference.target_id, entry.case_id],
                |r| r.get(0),
            )?;
            if !exists {
                return Err(Error::InvalidReference(reference.target_id.clone()));
            }
        }
        tx.execute(
            "INSERT INTO entries (id, case_id, record) VALUES (?1, ?2, ?3)",
            params![entry.id, entry.case_id, json],
        )?;
        for (position, reference) in entry.references.iter().enumerate() {
            tx.execute("INSERT INTO entry_references (source_id, case_id, position, target_id, relation) VALUES (?1, ?2, ?3, ?4, ?5)", params![entry.id, entry.case_id, position as i64, reference.target_id, reference.relation])?;
        }
        tx.commit()?;
        Ok(entry)
    }

    fn get(&self, id: &str) -> Result<Option<Entry>, Error> {
        valid_id(id)?;
        let json: Option<String> = self
            .connection
            .query_row("SELECT record FROM entries WHERE id=?1", [id], |r| r.get(0))
            .optional()?;
        json.map(|j| serde_json::from_str(&j).map_err(Error::from))
            .transpose()
    }

    fn history(
        &self,
        case_id: &str,
        after: Option<&str>,
        limit: usize,
    ) -> Result<HistoryPage, Error> {
        valid_id(case_id)?;
        if !(1..=1000).contains(&limit) {
            return Err(Error::Validation("limit must be in 1..=1000".into()));
        }
        let cursor = match after {
            Some(value) => value
                .parse::<i64>()
                .ok()
                .filter(|n| *n > 0 && n.to_string() == value)
                .ok_or_else(|| Error::Validation("invalid history cursor".into()))?,
            None => 0,
        };
        if self.get_case(case_id)?.is_none() {
            return Err(Error::NotFound {
                entity: "case",
                id: case_id.into(),
            });
        }
        let mut statement = self.connection.prepare(
            "SELECT seq, record FROM entries WHERE case_id=?1 AND seq>?2 ORDER BY seq LIMIT ?3",
        )?;
        let rows = statement.query_map(params![case_id, cursor, (limit + 1) as i64], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut records = rows.collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if records.len() > limit {
            records.pop();
            records.last().map(|r| r.0.to_string())
        } else {
            None
        };
        let entries = records
            .into_iter()
            .map(|(_, json)| serde_json::from_str(&json).map_err(Error::from))
            .collect::<Result<_, _>>()?;
        Ok(HistoryPage {
            entries,
            next_cursor,
        })
    }
}

const SCHEMA: &str = "
CREATE TABLE cases (id TEXT PRIMARY KEY NOT NULL, record TEXT NOT NULL);
CREATE TABLE entries (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    case_id TEXT NOT NULL REFERENCES cases(id),
    record TEXT NOT NULL,
    UNIQUE(id, case_id)
);
CREATE INDEX entries_history ON entries(case_id, seq);
CREATE TABLE entry_references (
    source_id TEXT NOT NULL,
    case_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    target_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    PRIMARY KEY(source_id, position),
    UNIQUE(source_id, target_id, relation),
    FOREIGN KEY(source_id, case_id) REFERENCES entries(id, case_id),
    FOREIGN KEY(target_id, case_id) REFERENCES entries(id, case_id)
);
CREATE TRIGGER cases_no_update BEFORE UPDATE ON cases BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER cases_no_delete BEFORE DELETE ON cases BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER entries_no_update BEFORE UPDATE ON entries BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER entries_no_delete BEFORE DELETE ON entries BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER references_no_update BEFORE UPDATE ON entry_references BEGIN SELECT RAISE(ABORT, 'append-only'); END;
CREATE TRIGGER references_no_delete BEFORE DELETE ON entry_references BEGIN SELECT RAISE(ABORT, 'append-only'); END;
";

#[cfg(test)]
mod tests;
