pub const FORBIDDEN: [&str; 8] = [
    "\0", "\u{1f}", "\u{7f}", "\u{85}", "\u{202e}", "\u{2066}", "\u{200b}", "\u{feff}",
];

/// Replace clear CHAT/ANNOUNCE text fields, preserving all other raw fields.
/// Callers independently re-sign; this helper grants no trust.
pub fn replace(raw: &[u8], nickname: &str, message: Option<&str>) -> Vec<u8> {
    let mut out = raw.to_vec();
    let nick_at = if raw[1] == 1 {
        32
    } else {
        assert_eq!(raw[1], 2);
        34
    };
    let old_nick = usize::from(raw[nick_at - 1]);
    out.splice(nick_at..nick_at + old_nick, nickname.bytes());
    out[nick_at - 1] = nickname.len() as u8;
    if let Some(message) = message {
        assert_eq!(raw[1], 1);
        let len_at = nick_at + nickname.len() + 4;
        let old_len = usize::from(u16::from_be_bytes(
            out[len_at..len_at + 2].try_into().unwrap(),
        ));
        out.splice(len_at + 2..len_at + 2 + old_len, message.bytes());
        out[len_at..len_at + 2].copy_from_slice(&(message.len() as u16).to_be_bytes());
    }
    let len = (out.len() - 26) as u16;
    out[24..26].copy_from_slice(&len.to_be_bytes());
    out
}
