//! Lecture config SMTP — priorités alignées sur `getPlatformSmtpConfig` (Next).

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// Adresse From (MAILER_EMAIL / SMTP_USER).
    pub from: String,
    /// Username auth SMTP (peut différer du From — TEM).
    pub auth_user: String,
    pub pass: String,
    pub host: String,
    pub port: u16,
    /// TLS implicite (465) vs STARTTLS (587).
    pub secure: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SmtpConfigError {
    #[error("SMTP: adresse From absente (MAILER_EMAIL / SMTP_USER)")]
    MissingFrom,
    #[error("SMTP: mot de passe absent (MAILER_SMTP_PASS / SMTP_PASS)")]
    MissingPass,
    #[error("SMTP: host absent (MAILER_SMTP_HOST / SMTP_HOST)")]
    MissingHost,
}

impl SmtpConfig {
    pub fn from_env() -> Result<Self, SmtpConfigError> {
        let from = first_nonempty(&[
            "MAILER_EMAIL",
            "TRAVEL_INBOUND_EMAIL",
            "SMTP_USER",
        ])
        .ok_or(SmtpConfigError::MissingFrom)?;

        let smtp_host = first_nonempty(&["MAILER_SMTP_HOST", "SMTP_HOST"]);
        let smtp_user = first_nonempty(&["MAILER_SMTP_USER", "SMTP_USER"]);
        let smtp_pass = first_nonempty(&["MAILER_SMTP_PASS", "SMTP_PASS"]);

        // TEM / SMTP explicite : host + user auth + pass.
        if let (Some(host), Some(auth_user), Some(pass)) =
            (smtp_host.clone(), smtp_user.clone(), smtp_pass.clone())
        {
            if !host.is_empty() && !auth_user.is_empty() && !pass.is_empty() {
                let port = parse_port(
                    first_nonempty(&["MAILER_SMTP_PORT", "SMTP_PORT"]).as_deref(),
                    if is_tem_host(&host) { 587 } else { default_port_for_host(&host, &from) },
                );
                let secure = parse_secure(
                    first_nonempty(&["MAILER_SMTP_SECURE", "SMTP_SECURE"]).as_deref(),
                    port,
                );
                return Ok(Self {
                    from,
                    auth_user,
                    pass,
                    host,
                    port,
                    secure,
                });
            }
        }

        // Legacy : host plateforme + pass.
        let host = smtp_host
            .or_else(|| first_nonempty(&["MAILER_HOST", "TRAVEL_IMAP_HOST", "IMAP_HOST"]))
            .or_else(|| infer_host_from_from(&from))
            .ok_or(SmtpConfigError::MissingHost)?;
        let pass = smtp_pass
            .or_else(|| first_nonempty(&["MAILER_PASS", "TRAVEL_IMAP_PASS", "IMAP_PASS"]))
            .ok_or(SmtpConfigError::MissingPass)?;

        let port = parse_port(
            first_nonempty(&["MAILER_SMTP_PORT", "SMTP_PORT"]).as_deref(),
            default_port_for_host(&host, &from),
        );
        let secure = parse_secure(
            first_nonempty(&["MAILER_SMTP_SECURE", "SMTP_SECURE"]).as_deref(),
            port,
        );
        let auth_user = smtp_user.unwrap_or_else(|| from.clone());

        Ok(Self {
            from,
            auth_user,
            pass,
            host,
            port,
            secure,
        })
    }
}

fn first_nonempty(keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Ok(v) = std::env::var(k) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn parse_port(raw: Option<&str>, fallback: u16) -> u16 {
    raw.and_then(|s| s.trim().parse::<u16>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(fallback)
}

fn parse_secure(raw: Option<&str>, port: u16) -> bool {
    if let Some(s) = raw {
        let t = s.trim();
        if t == "false" || t == "0" {
            return false;
        }
        if !t.is_empty() {
            return true;
        }
    }
    port == 465 || port == 2465 || port == 993
}

fn is_tem_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h.ends_with("tem.scaleway.com") || h.contains("smtp.tem.")
}

/// Heuristique Gmail / Outlook si host non fourni (cas `.env.local` USER+PASS seuls).
fn infer_host_from_from(from: &str) -> Option<String> {
    let lower = from.to_ascii_lowercase();
    if lower.ends_with("@gmail.com") || lower.ends_with("@googlemail.com") {
        return Some("smtp.gmail.com".into());
    }
    if lower.ends_with("@outlook.com")
        || lower.ends_with("@hotmail.com")
        || lower.ends_with("@live.com")
    {
        return Some("smtp.office365.com".into());
    }
    None
}

fn default_port_for_host(host: &str, from: &str) -> u16 {
    let h = host.to_ascii_lowercase();
    if h.contains("gmail") || from.to_ascii_lowercase().contains("@gmail") {
        587
    } else if is_tem_host(host) {
        587
    } else {
        465
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_gmail() {
        assert_eq!(
            infer_host_from_from("flojfk@gmail.com").as_deref(),
            Some("smtp.gmail.com")
        );
    }
}
