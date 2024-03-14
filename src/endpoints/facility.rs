use crate::shared::{
    sql::{self, Certification, Controller},
    AppError, AppState, UserInfo, SESSION_USER_INFO_KEY,
};
use axum::{extract::State, response::Html, routing::get, Router};
use itertools::Itertools;
use minijinja::{context, Environment};
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};
use tower_sessions::Session;

const STAFF_ROLES: [&str; 11] = [
    "ATM", "DATM", "TA", "FE", "EC", "WM", "INS", "MTR", "AFE", "AEC", "AWM",
];

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
    let mut staff: HashMap<&str, Vec<Controller>> = HashMap::from([
        (STAFF_ROLES[0], Vec::new()),
        (STAFF_ROLES[1], Vec::new()),
        (STAFF_ROLES[2], Vec::new()),
        (STAFF_ROLES[3], Vec::new()),
        (STAFF_ROLES[4], Vec::new()),
        (STAFF_ROLES[5], Vec::new()),
        (STAFF_ROLES[6], Vec::new()),
        (STAFF_ROLES[7], Vec::new()),
        (STAFF_ROLES[8], Vec::new()),
        (STAFF_ROLES[9], Vec::new()),
        (STAFF_ROLES[10], Vec::new()),
    ]);
    let controllers: Vec<Controller> = sqlx::query_as(sql::GET_ALL_CONTROLLERS)
        .fetch_all(&state.db)
        .await?;
    for controller in controllers {
        let roles = controller.roles.split(',').collect::<Vec<_>>();
        for role in roles {
            if staff.contains_key(role) {
                let ovr = state
                    .config
                    .staff
                    .overrides
                    .iter()
                    .find(|ovr| ovr.role == role);
                if let Some(ovr) = ovr {
                    if ovr.cid == controller.cid {
                        (*staff.get_mut(role).unwrap()).push(controller.clone());
                    } else {
                        let role = format!("A{role}");
                        (*staff.get_mut(role.as_str()).unwrap()).push(controller.clone());
                    }
                } else {
                    (*staff.get_mut(role).unwrap()).push(controller.clone());
                }
            }
        }
        if controller.home_facility == state.config.vatsim.vatusa_facility_code
            && [8, 9, 10].contains(&controller.rating)
        {
            (*staff.get_mut("INS").unwrap()).push(controller.clone());
        }
    }

    let email_domain = &state.config.staff.email_domain;
    let has_email = HashMap::from([
        (STAFF_ROLES[0], true),
        (STAFF_ROLES[1], true),
        (STAFF_ROLES[2], true),
        (STAFF_ROLES[3], true),
        (STAFF_ROLES[4], true),
        (STAFF_ROLES[5], true),
        (STAFF_ROLES[6], false),
        (STAFF_ROLES[7], false),
        (STAFF_ROLES[8], false),
        (STAFF_ROLES[9], false),
        (STAFF_ROLES[10], false),
    ]);
    let user_info: Option<UserInfo> = session.get(SESSION_USER_INFO_KEY).await?;
    let template = state.templates.get_template("staff")?;
    let rendered = template.render(context! {
       user_info,
       staff,
       roles => STAFF_ROLES,
       email_domain,
       has_email,
    })?;
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
