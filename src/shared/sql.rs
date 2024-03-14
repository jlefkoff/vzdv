use sqlx::{
    prelude::{FromRow, Row},
    sqlite::SqliteRow,
    types::{
        chrono::{DateTime, Utc},
        Uuid,
    },
};

#[derive(Debug, FromRow)]
pub struct Controller {
    pub id: u32,
    pub cid: u32,
    pub first_name: String,
    pub last_name: String,
    pub operating_initials: Option<String>,
    pub rating: u8,
    pub status: String,
    pub discord_id: Option<String>,
    pub home_facility: String,
    pub roles: String,
    pub created_date: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
pub struct Certification {
    pub id: u32,
    pub controller_id: u32,
    pub name: String,
    pub value: String,
    pub changed_on: DateTime<Utc>,
    pub set_by: u32,
}

/// Statements to create tables. Only ran when the DB file does not exist,
/// so no migration or "IF NOT EXISTS" conditions need to be added.
pub const CREATE_TABLES: &str = r#"
CREATE TABLE controller (
    id INTEGER PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT,
    operating_initials TEXT UNIQUE,
    rating INTEGER,
    status TEXT,
    discord_id TEXT UNIQUE,
    home_facility TEXT,
    roles TEXT,
    created_date TEXT
);

CREATE TABLE certification (
    id INTEGER PRIMARY KEY NOT NULL,
    controller_id TEXT NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    changed_on TEXT NOT NULL,
    set_by INTEGER NOT NULL,

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

pub const UPSERT_USER_LOGIN: &str = "
INSERT INTO controller
    (id, cid, first_name, last_name, email)
VALUES
    (NULL, $1, $2, $3, $4)
ON CONFLICT(cid) DO UPDATE SET
    first_name=excluded.first_name,
    last_name=excluded.last_name,
    email=excluded.email
WHERE
    cid=excluded.cid;
";

pub const INSERT_FEEDBACK: &str = "
INSERT INTO feedback
    (id, controller, position, rating, comments, created_date, submitter_cid)
VALUES
    (NULL, $1, $2, $3, $4, $5, $6);
";

pub const USER_LOOKUP: &str = "SELECT * FROM controller WHERE cid=$1;";

pub const UPSERT_USER_TASK: &str = "
INSERT INTO controller
    (id, cid, first_name, last_name, email, rating, home_facility, roles, created_date)
VALUES
    (NULL, $1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT(cid) DO UPDATE SET
    first_name=excluded.first_name,
    last_name=excluded.last_name,
    email=excluded.email,
    rating=excluded.rating,
    home_facility=excluded.home_facility,
    roles=excluded.roles
WHERE
    cid=excluded.cid;
";
