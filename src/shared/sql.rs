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
    pub is_on_roster: bool,
    pub roles: String,
    pub loa_until: Option<DateTime<Utc>>,
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
    /// "Training", "Solo", "Certified"
    pub value: String,
    pub changed_on: DateTime<Utc>,
    pub set_by: u32,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Activity {
    pub id: u32,
    pub cid: u32,
    pub month: String,
    pub minutes: u32,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Feedback {
    pub id: u32,
    pub controller: String,
    pub position: String,
    pub rating: String,
    pub comments: String,
    pub created_date: DateTime<Utc>,
    pub submitter_cid: u32,
    pub reviewed_by_cid: u32,
    pub posted_to_discord: bool,
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
    is_on_roster BOOLEAN,
    roles TEXT,
    loa_until TEXT
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
    reviewer_action TEXT,
    posted_to_discord BOOLEAN NOT NULL DEFAULT FALSE
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
    (id, cid, first_name, last_name, email, is_on_roster)
VALUES
    (NULL, $1, $2, $3, $4, FALSE)
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
    (id, cid, first_name, last_name, email, rating, home_facility, is_on_roster, roles, created_date)
VALUES
    (NULL, $1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT(cid) DO UPDATE SET
    first_name=excluded.first_name,
    last_name=excluded.last_name,
    email=excluded.email,
    rating=excluded.rating,
    home_facility=excluded.home_facility,
    is_on_roster=excluded.is_on_roster,
    roles=excluded.roles
WHERE
    cid=excluded.cid
";

pub const GET_ALL_CONTROLLERS: &str = "SELECT * FROM controller";
pub const GET_ALL_CONTROLLERS_ON_ROSTER: &str = "SELECT * FROM controller WHERE is_on_roster=TRUE";
pub const GET_ALL_CONTROLLER_CIDS: &str = "SELECT cid FROM controller";
pub const GET_ALL_ROSTER_CONTROLLER_CIDS: &str =
    "SELECT cid FROM controller WHERE is_on_roster=TRUE";
pub const UPDATE_REMOVED_FROM_ROSTER: &str = "UPDATE controller SET is_on_roster=0 WHERE cid=$1";
pub const GET_CONTROLLER_BY_CID: &str = "SELECT * FROM controller WHERE cid=$1";
pub const GET_CONTROLLER_CIDS_AND_NAMES: &str = "SELECT cid, first_name, last_name from controller";

pub const GET_ALL_CERTIFICATIONS: &str = "SELECT * FROM certification";

pub const GET_ALL_ACTIVITY: &str = "SELECT * FROM activity";
pub const DELETE_ALL_ACTIVITY: &str = "DELETE FROM activity";
pub const INSERT_INTO_ACTIVITY: &str = "
INSERT INTO activity
    (id, cid, month, minutes)
VALUES
    (NULL, $1, $2, $3)
";

pub const GET_ALL_PENDING_FEEDBACK: &str = "SELECT * FROM feedback WHERE reviewed_by_cid IS NULL";
pub const GET_FEEDBACK_BY_ID: &str = "SELECT * FROM feedback WHERE id=$1";
pub const UPDATE_FEEDBACK_IGNORE: &str =
    "UPDATE feedback SET reviewed_by_cid=$1, reviewer_action=$2, posted_to_discord=$3";
