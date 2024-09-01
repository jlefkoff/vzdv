#![allow(unused)] // TODO

use crate::shared::AppError;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use vzdv::config::Config;

/// Send an SMTP email to the recipient.
pub fn send_mail(
    config: &Config,
    recipient: &str,
    subject: &str,
    body: &str,
) -> Result<(), AppError> {
    let email = Message::builder()
        .from(config.email.from.parse().unwrap())
        .reply_to(config.email.reply_to.parse().unwrap())
        .to(recipient.parse().unwrap())
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(body.to_owned())
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
