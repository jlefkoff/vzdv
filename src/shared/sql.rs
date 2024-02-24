use sqlx::{
    prelude::FromRow,
    types::{
        chrono::{DateTime, Utc},
        Uuid,
    },
};

#[derive(Debug, FromRow)]
pub struct Controller {
    pub id: Uuid,
    pub cid: u32,
    pub first_name: String,
    pub last_name: String,
    pub operating_initials: Option<String>,
    pub rating: String,
    pub status: String,
    pub discord_id: Option<u64>,
    pub join_date: Option<DateTime<Utc>>,
    pub staff_positions: Option<Vec<String>>,
}

#[derive(Debug, FromRow)]
pub struct Certification {
    pub id: Uuid,
    pub controller_id: Uuid,
    pub name: String,
    pub value: String,
    pub changed_on: DateTime<Utc>,
    pub set_by: Uuid,
}

/// Statements to create tables. Only ran when the DB file does not exist,
/// so no migration or "IF NOT EXISTS" conditions need to be added.
pub const CREATE_TABLES: &str = "
CREATE TABLE controller (
    id TEXT PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    operating_initials TEXT UNIQUE,
    rating TEXT NOT NULL,
    status TEXT NOT NULL,
    discord_id INTEGER UNIQUE,
    join_date TEXT,
    staff_positions TEXT
);

CREATE TABLE certification (
    id TEXT PRIMARY KEY NOT NULL,
    controller_id TEXT NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    changed_on TEXT NOT NULL,
    set_by TEXT NOT NULL,

    FOREIGN KEY (controller_id) REFERENCES controller(id)
);
";
