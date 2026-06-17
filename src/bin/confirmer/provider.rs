use parmail::email::Email;

/// Universal confirmation data extracted from any forwarding confirmation email.
#[derive(Debug, Clone)]
pub struct Confirmation {
    pub originator: String,
    pub confirm_url: String,
}

/// Provider detection, extraction, and template association.
pub struct Provider {
    pub from_address: &'static str,
    pub template: &'static str,
    pub extract: fn(&Email) -> Option<Confirmation>,
}

impl Provider {
    pub fn detect(&self, email: &Email) -> bool {
        email.info.from_address == self.from_address
    }

    pub fn render(&self, name: &str, confirmation: &Confirmation) -> String {
        self.template
            .replace("{originator}", &confirmation.originator)
            .replace("{confirm_url}", &confirmation.confirm_url)
            .replace("{provider}", name)
    }
}

static DEFAULT_PROVIDER: Provider = Provider {
    from_address: "",
    template: include_str!("templates/confirm.txt"),
    extract: |_| None,
};

/// All known providers, keyed by name.
static PROVIDERS: &[(&str, Provider)] = &[
    ("Gmail", Provider {
        from_address: "forwarding-noreply@google.com",
        template: include_str!("templates/gmail.txt"),
        extract: gmail::extract,
    }),
    ("O365", Provider {
        from_address: "noreply@microsoft.com",
        template: include_str!("templates/confirm.txt"),
        extract: o365::extract,
    }),
];

/// Is this email a forwarding request from any known provider?
pub fn is_forwarding_request(email: &Email) -> bool {
    get_forwarding_provider(email).is_some()
}

/// Identify which provider sent this forwarding request.
pub fn get_forwarding_provider(email: &Email) -> Option<(&'static str, &'static Provider)> {
    PROVIDERS.iter()
        .find(|(_, provider)| provider.detect(email))
        .map(|(name, provider)| (*name, provider))
}

/// Look up a provider by name, falling back to the default.
pub fn get_provider(name: &str) -> &'static Provider {
    PROVIDERS.iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
        .unwrap_or(&DEFAULT_PROVIDER)
}

mod gmail {
    use regex::Regex;
    use super::Confirmation;
    use parmail::email::Email;

    pub fn extract(email: &Email) -> Option<Confirmation> {
        let body = email.body.as_deref()?;

        let originator_re = Regex::new(r"(?m)^(\S+@\S+) has requested to automatically forward")
            .expect("invalid regex");
        let originator = originator_re
            .captures(body)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())?;

        let url_re = Regex::new(r"(https://mail(?:-settings)?\.google\.com/mail/vf-\S+)")
            .expect("invalid regex");
        let confirm_url = url_re
            .captures(body)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())?;

        Some(Confirmation {
            originator,
            confirm_url,
        })
    }
}

mod o365 {
    use regex::Regex;
    use super::Confirmation;
    use parmail::email::Email;

    pub fn extract(email: &Email) -> Option<Confirmation> {
        let body = email.body.as_deref()?;

        let originator_re = Regex::new(r"(\S+@\S+).*requested.*forward")
            .expect("invalid regex");
        let originator = originator_re
            .captures(body)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())?;

        let url_re = Regex::new(r"(https://\S+)")
            .expect("invalid regex");
        let confirm_url = url_re
            .captures(body)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())?;

        Some(Confirmation {
            originator,
            confirm_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parmail::email::parse_email;

    #[test]
    fn test_detect_gmail_forwarding_original() {
        let raw = include_bytes!("../../../emails/hmbkiiso30pa3lvcrgma9m4rss3a9fr51uoi6781");
        let parsed = parse_email(raw).unwrap();
        assert!(is_forwarding_request(&parsed));
        let (name, provider) = get_forwarding_provider(&parsed).unwrap();
        assert_eq!(name, "Gmail");
        let confirmation = (provider.extract)(&parsed).unwrap();
        assert_eq!(confirmation.originator, "sickofm23@gmail.com");
        assert!(confirmation.confirm_url.starts_with("https://mail-settings.google.com/mail/vf-"));
    }

    #[test]
    fn test_detect_gmail_forwarding_recent() {
        let raw = include_bytes!("../../../test_data/confirm_1.eml");
        let parsed = parse_email(raw).unwrap();
        assert!(is_forwarding_request(&parsed));
        let (name, provider) = get_forwarding_provider(&parsed).unwrap();
        assert_eq!(name, "Gmail");
        let confirmation = (provider.extract)(&parsed).unwrap();
        assert_eq!(confirmation.originator, "christian.bongiorno@gmail.com");
        assert!(confirmation.confirm_url.contains("/mail/vf-"), "unexpected URL: {}", confirmation.confirm_url);
    }

    #[test]
    fn test_regular_email_not_detected() {
        let email = Email {
            info: parmail::email::Header {
                subject: "Your Daily Digest for Mon, Jun 16".to_string(),
                from: "USPS Informed Delivery".to_string(),
                from_address: "christian.bongiorno@gmail.com".to_string(),
                date: "2026-06-16T19:00:00Z".to_string(),
                message_id: "test@example.com".to_string(),
            },
            body: Some("Here is your daily mail scan.".to_string()),
            images: vec![],
        };
        assert!(!is_forwarding_request(&email));
        assert!(get_forwarding_provider(&email).is_none());
    }
}
