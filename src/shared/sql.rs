use serde::Serialize;
use sqlx::{
    prelude::{FromRow, Row},
    sqlite::SqliteRow,
    types::{
        chrono::{DateTime, Utc},
        Uuid,
    },
};

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct Controller {
    pub id: u32,
    pub cid: u32,
    pub first_name: String,
    pub last_name: String,
    pub operating_initials: Option<String>,
    pub rating: i8,
    pub status: String,
    pub discord_id: Option<String>,
    pub home_facility: String,
    pub roles: String,
    pub created_date: Option<DateTime<Utc>>,
}

impl Controller {
    /// Friendly name for the controller's numeric rating.
    pub fn rating_name(rating: i8) -> &'static str {
        match rating {
            -1 => "INA",
            0 => "SUS",
            1 => "OBS",
            2 => "S1",
            3 => "S2",
            4 => "S3",
            5 => "C1",
            6 => "C2",
            7 => "C3",
            8 => "I1",
            9 => "I2",
            10 => "I3",
            11 => "SUP",
            12 => "ADM",
            _ => "???",
        }
    }
}

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct Certification {
    pub id: u32,
    pub cid: u32,
    pub name: String,
    /// "In Progress", "Solo", "Certified"
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
    cid INTEGER NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    changed_on TEXT NOT NULL,
    set_by INTEGER NOT NULL
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

CREATE TABLE activity (
    id INTEGER PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL,
    month TEXT NOT NULL,
    minutes INTEGER NOT NULL,

    FOREIGN KEY (cid) REFERENCES controller(cid)
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
    cid=excluded.cid
";

pub const INSERT_FEEDBACK: &str = "
INSERT INTO feedback
    (id, controller, position, rating, comments, created_date, submitter_cid)
VALUES
    (NULL, $1, $2, $3, $4, $5, $6)
";

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
    cid=excluded.cid
";

pub const GET_ALL_CONTROLLERS: &str = "SELECT * FROM controller";
pub const GET_ALL_CONTROLLER_CIDS: &str = "SELECT cid FROM controller";

pub const GET_ALL_CERTIFICATIONS: &str = "SELECT * FROM certification";

pub const DELETE_FROM_ACTIVITY: &str = "DELETE FROM activity WHERE cid=$1";
pub const INSERT_INTO_ACTIVITY: &str = "
INSERT INTO activity
    (id, cid, month, minutes)
VALUES
    (NULL, $1, $2, $3)
";

pub const DELETE_FROM_ROSTER: &str = "DELETE FROM controller WHERE cid=$1";
