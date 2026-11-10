#![forbid(unsafe_code)]

mod model;
mod repository;
mod sqlite;

pub use model::{Case, Entry, HistoryPage, MAX_RECORD_BYTES, NewCase, NewEntry, Reference};
pub use repository::{Error, Repository};
pub use sqlite::SqliteRepository;
