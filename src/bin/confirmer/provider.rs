use phf::phf_map;

use parmail::email::Email;

/// Universal confirmation data extracted from any forwarding confirmation email.
#[derive(Debug, Clone)]
pub struct Confirmation {
    pub originator: String,
    pub confirm_url: String,
}

/// Provider detection, extraction, and template association.
pub struct Provider {
    pub template: &'static str,
    pub detect: fn(&Email) -> bool,
    pub extract: fn(&Email) -> Option<Confirmation>,
}

impl Provider {
    pub fn render(&self, name: &str, confirmation: &Confirmation) -> String {
        self.template
            .replace("{originator}", &confirmation.originator)
            .replace("{confirm_url}", &confirmation.confirm_url)
            .replace("{provider}", name)
    }
}

static DEFAULT_PROVIDER: Provider = Provider {
    template: include_str!("templates/confirm.txt"),
    detect: |_| false,
    extract: |_| None,
};

/// All known providers, keyed by name. O(1) lookup at runtime.
static PROVIDERS: phf::Map<&'static str, Provider> = phf_map! {
    "Gmail" => Provider {
        template: include_str!("templates/gmail.txt"),
        detect: gmail::detect,
        extract: gmail::extract,
    },
    "O365" => Provider {
        template: include_str!("templates/confirm.txt"),
        detect: o365::detect,
        extract: o365::extract,
    },
};

/// Is this email a forwarding request from any known provider?
pub fn is_forwarding_request(email: &Email) -> bool {
    PROVIDERS.values().any(|provider| (provider.detect)(email))
}

/// Identify which provider sent this forwarding request.
pub fn get_forwarding_provider(email: &Email) -> Option<(&'static str, &'static Provider)> {
    PROVIDERS.entries()
        .find(|(_, provider)| (provider.detect)(email))
        .map(|(name, provider)| (*name, provider))
}

/// Look up a provider by name, falling back to the default.
pub fn get_provider(name: &str) -> &'static Provider {
    PROVIDERS.get(name).unwrap_or(&DEFAULT_PROVIDER)
}

mod gmail {
    use regex::Regex;
    use super::Confirmation;
    use parmail::email::Email;

    pub fn detect(email: &Email) -> bool {
        let from_match = email.info.from_address == "forwarding-noreply@google.com";
        let subject_match = email.info.subject.contains("Forwarding Confirmation");
        from_match && subject_match
    }

    pub fn extract(email: &Email) -> Option<Confirmation> {
        let body = email.body.as_deref()?;

        let originator_re = Regex::new(r"^(\S+@\S+) has requested to automatically forward")
            .expect("invalid regex");
        let originator = originator_re
            .captures(body)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string())?;

        let url_re = Regex::new(r"(https://mail-settings\.google\.com/mail/vf-\S+)")
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

    pub fn detect(email: &Email) -> bool {
        let from_match = email.info.from_address.ends_with("@microsoft.com");
        let subject_match = email.info.subject.to_lowercase().contains("forwarding");
        from_match && subject_match
    }

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
