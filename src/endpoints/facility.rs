use crate::shared::{
    sql::{self, Certification, Controller},
    AppError, AppState, Config, UserInfo, SESSION_USER_INFO_KEY,
};
use axum::{extract::State, response::Html, routing::get, Router};
use itertools::Itertools;
use log::warn;
use minijinja::{context, Environment};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tower_sessions::Session;

const IGNORE_MISSING_STAFF_POSITIONS_FOR: [&str; 1] = ["FACCBT"];

#[derive(Debug, Serialize)]
struct StaffPosition {
    short: &'static str,
    name: &'static str,
    order: u8,
    controllers: Vec<Controller>,
    email: Option<String>,
    description: &'static str,
}

fn generate_staff_outline(config: &Config) -> HashMap<&'static str, StaffPosition> {
    let email_domain = &config.staff.email_domain;
    HashMap::from([
        ("ATM", StaffPosition {
            short: "ATM",
            name: "Air Traffic Manager",
            order: 1,
            controllers: Vec::new(),
            email: Some(format!("atm@{email_domain}")),
            description: "Responsible for the macro-management of the facility. Oversees day-to-day operations and ensures that the facility is running smoothly.",
        }),
        ("DATM", StaffPosition {
            short: "DATM",
            name: "Deputy Air Traffic Manager",
            order: 2,
            controllers: Vec::new(),
            email: Some(format!("datm@{email_domain}")),
            description: "Assists the Air Traffic Manager with the management of the facility. Acts as the Air Traffic Manager in their absence.",
        }),
        ("TA", StaffPosition {
            short: "TA",
            name: "Training Administrator",
            order: 3,
            controllers: Vec::new(),
            email: Some(format!("ta@{email_domain}")),
            description: "Responsible for overseeing and management of the facility's training program and staff.",
        }),
        ("FE", StaffPosition {
            short: "FE",
            name: "Facility Engineer",
            order: 4,
            controllers: Vec::new(),
            email: Some(format!("fe@{email_domain}")),
            description: "Responsible for the creation of sector files, radar client files, and other facility resources.",
        }),
        ("EC", StaffPosition {
            short: "EC",
            name: "Events Coordinator",
            order: 5,
            controllers: Vec::new(),
            email: Some(format!("ec@{email_domain}")),
            description: "Responsible for the planning, coordination and advertisement of facility events with neighboring facilities, virtual airlines, VATUSA, and VATSIM.",
        }),
        ("WM", StaffPosition {
            short: "WM",
            name: "Webmaster",
            order: 6,
            controllers: Vec::new(),
            email: Some(format!("wm@{email_domain}")),
            description: "Responsible for the management of the facility's website and technical infrastructure.",
        }),
        ("INS", StaffPosition {
            short: "INS",
            name: "Instructor",
            order: 7,
            controllers: Vec::new(),
            email: None,
            description: "Under direction of the Training Administrator, leads training and handles OTS Examinations.",
        }),
        ("MTR", StaffPosition {
            short: "MTR",
            name: "Mentor",
            order: 8,
            controllers: Vec::new(),
            email: None,
            description: "Under direction of the Training Administrator, helps train students and prepare them for OTS Examinations.",
        }),
        ("AFE", StaffPosition {
            short: "AFE",
            name: "Assistant Facility Engineer",
            order: 9,
            controllers: Vec::new(),
            email: None,
            description: "Assists the Facility Engineer.",
        }),
        ("AEC", StaffPosition {
            short: "AEC",
            name: "Assistant Events Coordinator",
            order: 10,
            controllers: Vec::new(),
            email: None,
            description: "Assists the Events Coordinator.",
        }),
        ("AWM", StaffPosition {
            short: "AWM",
            name: "Assistant Webmaster",
            order: 11,
            controllers: Vec::new(),
            email: None,
            description: "Assists the Webmaster.",
        }),
    ])
}

#[derive(Debug, Serialize)]
struct ControllerWithCerts<'a> {
    cid: u32,
    first_name: &'a str,
    last_name: &'a str,
    operating_initials: &'a str,
    rating: &'static str,
    is_home: bool,
    roles: String,
    certs: Vec<Certification>,
}

/// View the full roster.
async fn page_roster(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let controllers: Vec<Controller> = sqlx::query_as(sql::GET_ALL_CONTROLLERS)
        .fetch_all(&state.db)
        .await?;
    let certifications: Vec<Certification> = sqlx::query_as(sql::GET_ALL_CERTIFICATIONS)
        .fetch_all(&state.db)
        .await?;

    let controllers_with_certs: Vec<_> = controllers
        .iter()
        .map(|controller| {
            let operating_initials = match &controller.operating_initials {
                Some(s) => s,
                None => "",
            };
            let roles = controller.roles.split(',').collect::<Vec<_>>().join(", ");

            let certs = certifications
                .iter()
                .filter(|cert| cert.cid == controller.cid)
                .cloned()
                .collect::<Vec<_>>();

            ControllerWithCerts {
                cid: controller.cid,
                first_name: &controller.first_name,
                last_name: &controller.last_name,
                operating_initials,
                rating: Controller::rating_name(controller.rating),
                is_home: controller.home_facility == state.config.vatsim.vatusa_facility_code,
                roles,
                certs,
            }
        })
        .sorted_by(|a, b| Ord::cmp(&a.cid, &b.cid))
        .collect();

    let template = state.templates.get_template("roster")?;
    let rendered = template.render(context! {
       user_info,
       controllers => controllers_with_certs
    })?;
    Ok(Html(rendered))
}

/// View the facility's staff.
async fn page_staff(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> Result<Html<String>, AppError> {
    let mut staff_map = generate_staff_outline(&state.config);
    let controllers: Vec<Controller> = sqlx::query_as(sql::GET_ALL_CONTROLLERS)
        .fetch_all(&state.db)
        .await?;
    for controller in &controllers {
        let roles = controller.roles.split_terminator(',').collect::<Vec<_>>();
        for role in roles {
            let mut is_assistant = false;
            if let Some(role_override) =
                state.config.staff.overrides.iter().find(|o| o.role == role)
            {
                if role_override.cid != controller.cid {
                    is_assistant = true
                }
            }
            // VATUSA doesn't differentiate between e.g. the main FE and their assistants,
            // at least not at the API level. Need something to be able to differentiate.
            let role = if is_assistant {
                format!("A{role}")
            } else {
                role.to_string()
            };
            if let Some(staff_pos) = staff_map.get_mut(role.as_str()) {
                staff_pos.controllers.push(controller.clone());
            } else if !IGNORE_MISSING_STAFF_POSITIONS_FOR.contains(&role.as_str()) {
                warn!("No staff role found for: {role}");
            }
        }
        if controller.home_facility == state.config.vatsim.vatusa_facility_code
            && [8, 9, 10].contains(&controller.rating)
        {
            staff_map
                .get_mut("INS")
                .unwrap()
                .controllers
                .push(controller.clone());
        }
    }

    let staff: Vec<_> = staff_map
        .values()
        .sorted_by(|a, b| Ord::cmp(&a.order, &b.order))
        .collect();

    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let template = state.templates.get_template("staff")?;
    let rendered = template.render(context! { user_info, staff })?;
    Ok(Html(rendered))
}

pub fn router(templates: &mut Environment) -> Router<Arc<AppState>> {
    templates
        .add_template("roster", include_str!("../../templates/roster.jinja"))
        .unwrap();
    templates
        .add_template("staff", include_str!("../../templates/staff.jinja"))
        .unwrap();

    Router::new()
        .route("/facility/roster", get(page_roster))
        .route("/facility/staff", get(page_staff))
}
