//! Domain matching for autofill phishing resistance (spec §8.3).

/// Common two-level public suffixes (compact list covering major ccTLD
/// structures — a full PSL is overkill for matching, not parsing).
const TWO_LEVEL_SUFFIXES: &[&str] = &[
    "ac.jp", "co.jp", "ed.jp", "go.jp", "gr.jp", "lg.jp", "ne.jp", "or.jp",
    "ac.uk", "co.uk", "gov.uk", "me.uk", "org.uk",
    "asn.au", "com.au", "edu.au", "net.au", "org.au",
    "com.br", "com.cn", "com.hk", "com.mx", "com.sg", "com.tw",
    "co.in", "co.kr", "co.nz", "ac.nz", "com.ar", "co.za", "co.th", "com.vn",
    "ac.in", "edu.cn", "org.cn", "net.cn", "com.ph", "com.my", "co.id",
];

/// Registrable domain (eTLD+1 approximation) of a host, lowercased.
/// Returns the last two labels, or three when the last two form a known
/// two-level public suffix (e.g. `foo.example.co.jp` → `example.co.jp`).
pub fn registrable(host: &str) -> String {
    let h = host.trim().trim_end_matches('.').to_lowercase();
    let labels: Vec<&str> = h.split('.').filter(|l| !l.is_empty()).collect();
    let n = labels.len();
    if n <= 2 {
        return labels.join(".");
    }
    let last2 = format!("{}.{}", labels[n - 2], labels[n - 1]);
    if TWO_LEVEL_SUFFIXES.contains(&last2.as_str()) {
        return format!("{}.{}", labels[n - 3], last2);
    }
    last2
}

/// An item may autofill for `request_host` when its stored url's registrable
/// domain equals the request's. Never matches empty/mismatched domains.
pub fn matches(item_url: Option<&str>, request_host: &str) -> bool {
    let Some(url) = item_url else { return false };
    let item_host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', '?', ':'])
        .next()
        .unwrap_or("");
    !item_host.is_empty() && registrable(item_host) == registrable(request_host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registrable_domains() {
        assert_eq!(registrable("www.example.com"), "example.com");
        assert_eq!(registrable("login.example.co.jp"), "example.co.jp");
        assert_eq!(registrable("EXAMPLE.com."), "example.com");
        assert_eq!(registrable("localhost"), "localhost");
        assert_eq!(registrable("a.b.example.co.uk"), "example.co.uk");
    }

    #[test]
    fn match_rules() {
        assert!(matches(Some("https://www.example.com/a"), "example.com"));
        assert!(matches(Some("https://example.com"), "login.example.com"));
        assert!(!matches(Some("https://example.com"), "evil-example.com"));
        assert!(!matches(Some("https://example.com.evil.com"), "example.com"));
        assert!(matches(Some("https://www.example.co.jp"), "app.example.co.jp"));
        assert!(!matches(None, "example.com"));
    }
}
