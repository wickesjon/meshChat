//! Display copies remain separate from original authenticated UTF-8 bytes.
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Nickname,
    Message,
    EventName,
    CredentialLabel,
}
impl Kind {
    pub fn max_bytes(self) -> usize {
        match self {
            Self::Nickname => 20,
            Self::Message => 280,
            Self::EventName => 32,
            Self::CredentialLabel => 16,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("text outside UTF-8 byte limit")]
    Length,
    #[error("forbidden control or invisible character")]
    Forbidden,
}
pub struct DisplayText<'a> {
    pub original: &'a str,
    pub rendered: String,
    /// Required native text-widget layout; the core does not measure glyph width.
    pub single_line_ellipsis: bool,
}
impl DisplayText<'_> {
    pub fn changed(&self) -> bool {
        self.original != self.rendered
    }
}
pub fn forbidden(c: char) -> bool {
    matches!(c, '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200b}'..='\u{200d}' | '\u{feff}')
}
pub fn validate(raw: &str, kind: Kind) -> Result<(), Error> {
    if raw.len() > kind.max_bytes() || (raw.is_empty() && kind != Kind::CredentialLabel) {
        return Err(Error::Length);
    }
    if raw.chars().any(forbidden) {
        return Err(Error::Forbidden);
    }
    Ok(())
}
fn capped_nfc(raw: &str) -> String {
    let mut clean = String::new();
    let mut marks = 0;
    let mut base = false;
    // Count decomposed marks so a precomposed accent cannot evade the mark cap.
    for c in raw.nfd() {
        if is_combining_mark(c) {
            if !base || marks == 2 {
                continue;
            }
            marks += 1;
        } else {
            marks = 0;
            base = true;
        }
        clean.push(c);
    }
    clean.nfc().collect()
}
pub fn display(raw: &str, kind: Kind) -> Result<DisplayText<'_>, Error> {
    validate(raw, kind)?;
    let mut rendered = capped_nfc(raw);
    if kind == Kind::Nickname {
        rendered = rendered
            .chars()
            .filter(|c| !matches!(c, '✓' | '✔' | '☑'))
            .map(|c| {
                if matches!(c, '\u{2028}' | '\u{2029}') {
                    ' '
                } else {
                    c
                }
            })
            .collect();
        // Remove ASCII badge words without lowercasing the presented nickname.
        // Repeat because removal can expose another forbidden substring.
        loop {
            let lower = rendered.to_ascii_lowercase();
            let found = ["staff", "official"]
                .iter()
                .find_map(|word| lower.find(word).map(|at| (at, word.len())));
            if let Some((at, len)) = found {
                rendered.replace_range(at..at + len, "");
            } else {
                break;
            }
        }
        // Removal can orphan marks; cap again in the final presentation.
        rendered = capped_nfc(rendered.trim());
        if rendered.is_empty() {
            rendered = "unnamed".into();
        }
    }
    Ok(DisplayText {
        original: raw,
        rendered,
        single_line_ellipsis: kind == Kind::Nickname,
    })
}
/// For outgoing editors, also check normalized/sanitized wire bytes. Never
/// truncate UTF-8 or silently shorten an authenticated original to fit a limit.
pub fn outgoing(raw: &str, kind: Kind) -> Result<String, Error> {
    let result = display(raw, kind)?.rendered;
    validate(&result, kind)?;
    Ok(result)
}
/// TR39 skeleton comparison is an impersonation warning, never identity proof.
pub fn confusable_with_own(candidate: &str, own: &str) -> Result<bool, Error> {
    let candidate = display(candidate, Kind::Nickname)?.rendered;
    let own = display(own, Kind::Nickname)?.rendered;
    Ok(unicode_security::skeleton(&candidate).eq(unicode_security::skeleton(&own)))
}
