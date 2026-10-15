//! Transport SMTP async (lettre + rustls).

use lettre::message::{header::ContentType, Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::{MailError, OutgoingMail, SmtpConfig};

pub async fn send_smtp(
    cfg: &SmtpConfig,
    mail: &OutgoingMail,
) -> Result<Option<String>, MailError> {
    let builder = Message::builder()
        .from(
            cfg.from
                .parse()
                .map_err(|e| MailError::Smtp(format!("From invalide: {e}")))?,
        )
        .to(mail
            .to
            .parse()
            .map_err(|e| MailError::Smtp(format!("To invalide: {e}")))?)
        .subject(mail.subject.clone());

    let email = if mail.attachments.is_empty() {
        builder
            .header(ContentType::TEXT_PLAIN)
            .body(mail.body_text.clone())
            .map_err(|e| MailError::Smtp(e.to_string()))?
    } else {
        let mut multipart = MultiPart::mixed().singlepart(
            SinglePart::builder()
                .header(ContentType::TEXT_PLAIN)
                .body(mail.body_text.clone()),
        );
        for att in &mail.attachments {
            let ctype: ContentType = att
                .mime
                .parse()
                .unwrap_or(ContentType::parse("application/octet-stream").expect("octet"));
            multipart = multipart.singlepart(
                Attachment::new(att.filename.clone()).body(att.bytes.clone(), ctype),
            );
        }
        builder
            .multipart(multipart)
            .map_err(|e| MailError::Smtp(e.to_string()))?
    };

    let creds = Credentials::new(cfg.auth_user.clone(), cfg.pass.clone());
    let mailer = if cfg.secure {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
            .map_err(|e| MailError::Smtp(e.to_string()))?
            .port(cfg.port)
            .credentials(creds)
            .build()
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            .map_err(|e| MailError::Smtp(e.to_string()))?
            .port(cfg.port)
            .credentials(creds)
            .build()
    };

    let response = mailer
        .send(email)
        .await
        .map_err(|e| MailError::Smtp(e.to_string()))?;

    let message_id = response.message().next().map(|s| s.to_string());
    Ok(message_id)
}
