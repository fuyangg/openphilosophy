use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

/// Maximum serialized request or record size in UTF-8 bytes.
pub const MAX_RECORD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub target_id: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCase {
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub schema_version: u32,
    pub id: String,
    pub title: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewEntry {
    pub case_id: String,
    pub kind: String,
    pub content: String,
    #[serde(default)]
    pub references: Vec<Reference>,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub schema_version: u32,
    pub id: String,
    pub case_id: String,
    pub kind: String,
    pub content: String,
    pub references: Vec<Reference>,
    pub author: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPage {
    pub entries: Vec<Entry>,
    /// Decimal storage sequence; use only with this database and Case.
    pub next_cursor: Option<String>,
}

pub(crate) fn bounded(value: &str, name: &str, max: usize) -> Result<(), Error> {
    if value.trim().is_empty() || value.len() > max {
        return Err(Error::Validation(format!(
            "{name} must be nonblank and at most {max} UTF-8 bytes"
        )));
    }
    Ok(())
}

pub(crate) fn valid_id(value: &str) -> Result<(), Error> {
    let id = Uuid::parse_str(value).map_err(|_| Error::Validation("invalid UUID".into()))?;
    if id.to_string() != value {
        return Err(Error::Validation(
            "UUID must use canonical lowercase hyphenated form".into(),
        ));
    }
    Ok(())
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    let json = serde_json::to_string(value)?;
    if json.len() > MAX_RECORD_BYTES {
        return Err(Error::Validation("record exceeds 1 MiB".into()));
    }
    Ok(json)
}

fn now() -> Result<String, Error> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|e| Error::Validation(e.to_string()))
}

impl Case {
    pub(crate) fn create(input: NewCase) -> Result<Self, Error> {
        bounded(&input.title, "title", 512)?;
        Ok(Self {
            schema_version: 1,
            id: Uuid::new_v4().to_string(),
            title: input.title,
            created_at: now()?,
        })
    }
}

impl Entry {
    pub(crate) fn create(input: NewEntry) -> Result<Self, Error> {
        valid_id(&input.case_id)?;
        bounded(&input.kind, "kind", 128)?;
        bounded(&input.author, "author", 256)?;
        bounded(&input.content, "content", MAX_RECORD_BYTES)?;
        if input.references.len() > 128 {
            return Err(Error::Validation(
                "at most 128 references are allowed".into(),
            ));
        }
        let mut seen = HashSet::new();
        for reference in &input.references {
            valid_id(&reference.target_id)?;
            bounded(&reference.relation, "relation", 128)?;
            if !seen.insert((&reference.target_id, &reference.relation)) {
                return Err(Error::Validation("duplicate reference".into()));
            }
        }
        Ok(Self {
            schema_version: 1,
            id: Uuid::new_v4().to_string(),
            case_id: input.case_id,
            kind: input.kind,
            content: input.content,
            references: input.references,
            author: input.author,
            created_at: now()?,
        })
    }
}
