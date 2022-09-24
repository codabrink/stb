pub const SQLITE_DB: &'static str = "db.sqlite";
pub const EMBEDDING_DB: &'static str = "embeddings.sqlite";
pub use crate::model::*;
pub use anyhow::{bail, Result};
pub use rusqlite::{params, Connection, Row};
pub use serde::{Deserialize, Serialize};
pub use std::ops::Range;
pub use std::path::Path;
pub use std::rc::Rc;
