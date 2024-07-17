use serde::Serialize;
use sqlx::{
    prelude::FromRow,
    types::chrono::{DateTime, Utc},
};

#[derive(Debug, FromRow, Serialize, Clone, Default)]
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
    pub reviewer_action: String,
    pub posted_to_discord: bool,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Resource {
    pub id: u32,
    pub category: String,
    pub name: String,
    pub file_name: Option<String>,
    pub link: Option<String>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct VisitorApplication {
    pub id: u32,
    pub cid: u32,
    pub first_name: String,
    pub last_name: String,
    pub home_facility: String,
    pub rating: u8,
    pub date: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Event {
    pub id: u32,
    pub published: bool,
    pub complete: bool,
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub description: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct EventPosition {
    pub id: u32,
    pub event_id: u32,
    pub name: String,
    pub category: String,
    pub cid: Option<u32>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct EventRegistration {
    pub id: u32,
    pub event_id: u32,
    pub cid: u32,
    pub choice_1: u32,
    pub choice_2: u32,
    pub choice_3: u32,
    pub notes: Option<String>,
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
    operating_initials TEXT,
    rating INTEGER,
    status TEXT,
    discord_id TEXT UNIQUE,
    home_facility TEXT,
    is_on_roster INTEGER,
    roles TEXT,
    loa_until TEXT
) STRICT;

CREATE TABLE certification (
    id INTEGER PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    changed_on TEXT NOT NULL,
    set_by INTEGER NOT NULL
) STRICT;

CREATE TABLE feedback (
    id INTEGER PRIMARY KEY NOT NULL,
    controller TEXT NOT NULL,
    position TEXT NOT NULL,
    rating TEXT NOT NULL,
    comments TEXT,
    created_date TEXT NOT NULL,
    submitter_cid INTEGER NOT NULL,
    reviewed_by_cid INTEGER,
    reviewer_action TEXT NOT NULL DEFAULT 'pending',
    posted_to_discord INTEGER NOT NULL DEFAULT FALSE
) STRICT;

CREATE TABLE activity (
    id INTEGER PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL,
    month TEXT NOT NULL,
    minutes INTEGER NOT NULL,

    FOREIGN KEY (cid) REFERENCES controller(cid)
) STRICT;

CREATE TABLE resource (
    id INTEGER PRIMARY KEY NOT NULL,
    category TEXT NOT NULL,
    name TEXT NOT NULL,
    file_name TEXT,
    link TEXT,
    updated TEXT NOT NULL
) STRICT;

CREATE TABLE visitor_request (
    id INTEGER PRIMARY KEY NOT NULL,
    cid INTEGER NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    home_facility TEXT NOT NULL,
    rating INTEGER NOT NULL,
    date TEXT NOT NULL
) STRICT;

CREATE TABLE event (
    id INTEGER PRIMARY KEY NOT NULL,
    created_by INTEGER NOT NULL,
    published INTEGER NOT NULL DEFAULT FALSE,
    complete INTEGER NOT NULL DEFAULT FALSE,
    name TEXT NOT NULL,
    start TEXT NOT NULL,
    end TEXT NOT NULL,
    description TEXT,
    image_url TEXT,

    FOREIGN KEY (created_by) REFERENCES controller(id)
) STRICT;

CREATE TABLE event_position (
    id INTEGER PRIMARY KEY NOT NULL,
    event_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    cid INTEGER,

    FOREIGN KEY (event_id) REFERENCES event(id),
    FOREIGN KEY (cid) REFERENCES controller(cid)
) STRICT;

CREATE TABLE event_registration (
    id INTEGER PRIMARY KEY NOT NULL,
    event_id INTEGER NOT NULL,
    cid INTEGER NOT NULL,
    choice_1 INTEGER NOT NULL,
    choice_2 INTEGER NOT NULL,
    choice_3 INTEGER NOT NULL,
    notes TEXT,

    FOREIGN KEY (event_id) REFERENCES event(id),
    FOREIGN KEY (cid) REFERENCES controller(cid),
    FOREIGN KEY (choice_1) REFERENCES event_position(id),
    FOREIGN KEY (choice_2) REFERENCES event_position(id),
    FOREIGN KEY (choice_3) REFERENCES event_position(id)
) STRICT;
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

pub const UPSERT_USER_TASK: &str = "
INSERT INTO controller
    (id, cid, first_name, last_name, email, rating, home_facility, is_on_roster, roles)
VALUES
    (NULL, $1, $2, $3, $4, $5, $6, $7, $8)
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
pub const DELETE_ACTIVITY_FOR_CID: &str = "DELETE FROM activity WHERE cid=$1";
pub const INSERT_INTO_ACTIVITY: &str = "
INSERT INTO activity
    (id, cid, month, minutes)
VALUES
    (NULL, $1, $2, $3)
";

pub const INSERT_FEEDBACK: &str = "
INSERT INTO feedback
    (id, controller, position, rating, comments, created_date, submitter_cid)
VALUES
    (NULL, $1, $2, $3, $4, $5, $6)
";
pub const GET_ALL_PENDING_FEEDBACK: &str =
    "SELECT * FROM feedback WHERE reviewed_by_cid IS NULL OR reviewer_action='archive'";
pub const GET_FEEDBACK_BY_ID: &str = "SELECT * FROM feedback WHERE id=$1";
pub const UPDATE_FEEDBACK_TAKE_ACTION: &str =
    "UPDATE feedback SET reviewed_by_cid=$1, reviewer_action=$2, posted_to_discord=$3 WHERE id=$4";
pub const DELETE_FROM_FEEDBACK: &str = "DELETE FROM feedback WHERE id=$1";

pub const GET_ALL_RESOURCES: &str = "SELECT * FROM resource";

pub const GET_PENDING_VISITOR_REQ_FOR: &str = "SELECT * FROM visitor_request WHERE cid=$1";
pub const INSERT_INTO_VISITOR_REQ: &str =
    "INSERT INTO visitor_request VALUES (NULL, $1, $2, $3, $4, $5, $6);";

pub const GET_UPCOMING_EVENTS: &str = "SELECT * FROM event WHERE end > $1 AND published = TRUE";
pub const GET_EVENT: &str = "SELECT * FROM event WHERE id=$1";

pub const GET_EVENT_POSITIONS: &str = "SELECT * FROM event_position WHERE event_id=$1";

pub const GET_EVENT_REGISTRATIONS: &str = "SELECT * FROM event_registration WHERE event_id=$1";
