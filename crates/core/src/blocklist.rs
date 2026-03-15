//! URL blocklist shared with the browser extension.
//!
//! The source of truth is `shared/blocklist.json`, which is embedded at compile
//! time.  The matching logic mirrors `extension/rules.js`.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

const BLOCKLIST_JSON: &str = include_str!("../../../shared/blocklist.json");

pub struct Blocklist {
    /// Blocked domains grouped by dot-component count.
    blocked_by_depth: HashMap<usize, HashSet<String>>,
    /// Path prefixes that indicate checkout/auth flows.
    blocked_path_prefixes: Vec<String>,
}

#[derive(serde::Deserialize)]
struct BlocklistData {
    blocked_domains: Vec<String>,
    blocked_path_prefixes: Vec<String>,
    // login_domain_pattern is handled in code, not via regex
}

impl Blocklist {
    pub fn load() -> &'static Blocklist {
        static INSTANCE: OnceLock<Blocklist> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let data: BlocklistData =
                serde_json::from_str(BLOCKLIST_JSON).expect("invalid blocklist.json");

            let mut blocked_by_depth: HashMap<usize, HashSet<String>> = HashMap::new();
            for domain in &data.blocked_domains {
                let depth = domain.split('.').count();
                blocked_by_depth
                    .entry(depth)
                    .or_default()
                    .insert(domain.clone());
            }

            Blocklist {
                blocked_by_depth,
                blocked_path_prefixes: data.blocked_path_prefixes,
            }
        })
    }

    /// Check whether a URL should be blocked (banks, auth, checkout, etc.).
    pub fn is_url_blocked(&self, url: &str) -> bool {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return true,
        };

        let hostname = match parsed.host_str() {
            Some(h) => h.to_lowercase(),
            None => return true,
        };

        if self.is_domain_blocked(&hostname) {
            return true;
        }

        if is_login_domain(&hostname) {
            return true;
        }

        let path = parsed.path().to_lowercase();
        for prefix in &self.blocked_path_prefixes {
            if path == *prefix || path.starts_with(&format!("{prefix}/")) {
                return true;
            }
        }

        false
    }

    fn is_domain_blocked(&self, hostname: &str) -> bool {
        for (&depth, set) in &self.blocked_by_depth {
            if let Some(suffix) = domain_suffix(hostname, depth) {
                if set.contains(suffix) {
                    return true;
                }
            }
        }
        false
    }
}

/// Extract the last `n` dot-components from a hostname.
fn domain_suffix(hostname: &str, n: usize) -> Option<&str> {
    let mut dot = hostname.len();
    for i in 0..n {
        match hostname[..dot].rfind('.') {
            Some(pos) => dot = pos,
            None => {
                return if n == i + 1 {
                    Some(hostname)
                } else {
                    None
                };
            }
        }
    }
    Some(&hostname[dot + 1..])
}

/// Match login.(any).(tld) pattern without regex.
/// Starts with "login.", then exactly 2 more dot-separated components.
fn is_login_domain(hostname: &str) -> bool {
    let rest = match hostname.strip_prefix("login.") {
        Some(r) => r,
        None => return false,
    };
    // Must have exactly one dot remaining (two components)
    let parts: Vec<&str> = rest.split('.').collect();
    parts.len() == 2 && parts.iter().all(|p| !p.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_domain_match() {
        let bl = Blocklist::load();
        assert!(bl.is_url_blocked("https://chase.com/accounts"));
    }

    #[test]
    fn subdomain_suffix_match() {
        let bl = Blocklist::load();
        assert!(bl.is_url_blocked("https://www.chase.com/"));
    }

    #[test]
    fn deep_suffix_match() {
        let bl = Blocklist::load();
        assert!(bl.is_url_blocked("https://www.accounts.google.com/signin"));
    }

    #[test]
    fn non_blocked_domain_passes() {
        let bl = Blocklist::load();
        assert!(!bl.is_url_blocked("https://example.com/page"));
    }

    #[test]
    fn path_prefix_blocking() {
        let bl = Blocklist::load();
        assert!(bl.is_url_blocked("https://example.com/checkout"));
        assert!(bl.is_url_blocked("https://example.com/checkout/step2"));
        assert!(!bl.is_url_blocked("https://example.com/checkouts-list"));
    }

    #[test]
    fn login_domain_pattern() {
        let bl = Blocklist::load();
        assert!(bl.is_url_blocked("https://login.example.com/"));
        // login.example.co.uk has 3 components after "login.", not 2
        assert!(!bl.is_url_blocked("https://login.example.co.uk/"));
    }

    #[test]
    fn combined_is_url_blocked() {
        let bl = Blocklist::load();
        assert!(bl.is_url_blocked("https://paypal.com/send"));
        assert!(bl.is_url_blocked("https://shop.example.com/oauth"));
        assert!(!bl.is_url_blocked("https://news.ycombinator.com/item?id=123"));
    }

    #[test]
    fn domain_suffix_extraction() {
        assert_eq!(domain_suffix("www.accounts.google.com", 3), Some("accounts.google.com"));
        assert_eq!(domain_suffix("chase.com", 2), Some("chase.com"));
        assert_eq!(domain_suffix("com", 2), None);
        assert_eq!(domain_suffix("a.b", 2), Some("a.b"));
    }

    #[test]
    fn login_domain_check() {
        assert!(is_login_domain("login.example.com"));
        assert!(!is_login_domain("login.example.co.uk"));
        assert!(!is_login_domain("notlogin.example.com"));
        assert!(!is_login_domain("login."));
    }
}
