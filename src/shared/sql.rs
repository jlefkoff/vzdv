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
pub const CREATE_TABLES: &str = r#"
CREATE TABLE controller (
    id INTEGER PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT NOT NULL,
    operating_initials TEXT UNIQUE,
    rating TEXT,
    status TEXT,
    discord_id INTEGER UNIQUE,
    staff_positions TEXT,
    created_date TEXT
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

CREATE TABLE feedback (
    id INTEGER PRIMARY KEY NOT NULL,
    controller TEXT NOT NULL,
    position TEXT NOT NULL,
    rating TEXT NOT NULL,
    comments TEXT,
    created_date TEXT NOT NULL,
    submitter_cid INTEGER NOT NULL,
    reviewed_by_cid INTEGER,
    posted_to_discord BOOLEAN NOT NULL DEFAULT "FALSE"
);
"#;

pub const UPSERT_USER: &str = "
INSERT INTO controller
    (id, cid, first_name, last_name, email)
VALUES
    (NULL, ?, ?, ?, ?)
ON CONFLICT(cid) DO UPDATE SET
    first_name=excluded.first_name,
    last_name=excluded.last_name,
    email=excluded.email
WHERE cid=excluded.cid;
";

pub const INSERT_FEEDBACK: &str = "
INSERT INTO feedback
    (id, controller, position, rating, comments, created_date, submitter_cid)
VALUES
    (NULL, ?, ?, ?, ?, ?, ?);";
