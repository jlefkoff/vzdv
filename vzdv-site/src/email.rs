use crate::shared::AppError;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use minijinja::{context, Environment};
use sqlx::{Pool, Sqlite};
use vzdv::config::Config;
use vzdv::sql::{self, Controller};

/// Email templates.
pub mod templates {
    pub const VISITOR_ACCEPTED: &str = "visitor_accepted";
    pub const VISITOR_DENIED: &str = "visitor_denied";
    pub const VISITOR_REMOVED: &str = "visitor_removed";
}

/// Send an SMTP email to the recipient.
pub async fn send_mail(
    config: &Config,
    db: &Pool<Sqlite>,
    recipient_name: &str,
    recipient_address: &str,
    template_name: &str,
) -> Result<(), AppError> {
    // template match from config
    let template = match template_name {
        templates::VISITOR_ACCEPTED => &config.email.visitor_accepted_template,
        templates::VISITOR_DENIED => &config.email.visitor_denied_template,
        templates::VISITOR_REMOVED => &config.email.visitor_removed_template,
        _ => {
            return Err(AppError::UnknownEmailTemplate(template_name.to_owned()));
        }
    };

    // ATM and DATM names for signing
    let atm_datm: Vec<Controller> = sqlx::query_as(sql::GET_ATM_AND_DATM).fetch_all(db).await?;
    let atm = atm_datm
        .iter()
        .find(|controller| controller.roles.contains("ATM") && !controller.roles.contains("DATM"))
        .map(|controller| format!("{} {}, ATM", controller.first_name, controller.last_name))
        .unwrap_or_default();
    let datm = atm_datm
        .iter()
        .find(|controller| controller.roles.contains("DATM"))
        .map(|controller| format!("{} {}, DATM", controller.first_name, controller.last_name))
        .unwrap_or_default();

    // template load and render
    let mut env = Environment::new();
    env.add_template("body", &template.body)?;
    let body = env
        .get_template("body")?
        .render(context! { recipient_name, atm, datm })?;

    // construct and send email
    let email = Message::builder()
        .from(config.email.from.parse().unwrap())
        .reply_to(config.email.reply_to.parse().unwrap())
        .to(recipient_address.parse().unwrap())
        .subject(template.subject.to_owned())
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .unwrap();
    let creds = Credentials::new(
        config.email.user.to_owned(),
        config.email.password.to_owned(),
    );
    let mailer = SmtpTransport::relay(&config.email.host)
        .unwrap()
        .credentials(creds)
        .build();
    mailer.send(&email)?;
    Ok(())
}
