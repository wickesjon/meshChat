use meshchat_core::{
    channel, links,
    text::{self, Kind},
};
fn next(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}
#[test]
fn bounded_link_and_text_mutations() {
    let corpus = [
        "meshfest://j/melodic-techno-valley",
        "https://meshfest.app/j/melodic-techno-valley",
        "meshfest://friend/AA/N",
        "meshfest://event/AA/E",
        "meshfest://staff/AA/AA",
        "e\u{301}",
        "sсоре",
        "✓STAFF",
        "🌻",
        "a\u{202e}b",
    ];
    let mut corpus: Vec<&str> = corpus.to_vec();
    corpus.extend(
        include_str!("../vectors/channels/public-links.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .map(|line| line.split('\t').nth(2).unwrap()),
    );
    let mut state = 0x4d43_3031_315f_7631;
    for step in 0..100_000 {
        let mut data = corpus[next(&mut state) as usize % corpus.len()]
            .as_bytes()
            .to_vec();
        for _ in 0..next(&mut state) % 8 {
            let at = next(&mut state) as usize % (data.len() + 1);
            match next(&mut state) % 4 {
                0 if data.len() < 2049 => data.insert(at, next(&mut state) as u8),
                1 if at < data.len() => {
                    data.remove(at);
                }
                2 if at < data.len() => data[at] ^= next(&mut state) as u8,
                _ => data.truncate(at),
            }
        }
        if step % 100 == 0 {
            data.resize(2049, b'A');
        }
        if let Ok(raw) = std::str::from_utf8(&data) {
            let _ = links::parse(raw);
            let _ = channel::parse_name(raw);
            for kind in [
                Kind::Nickname,
                Kind::Message,
                Kind::EventName,
                Kind::CredentialLabel,
            ] {
                if let Ok(display) = text::display(raw, kind) {
                    assert_eq!(display.original.as_bytes(), data);
                    assert!(!display.rendered.chars().any(text::forbidden));
                    assert!(display.rendered.len() <= kind.max_bytes() * 4 + 16);
                    use unicode_normalization::UnicodeNormalization;
                    assert!(display.rendered.nfc().eq(display.rendered.chars()));
                }
                let _ = text::outgoing(raw, kind);
            }
            let _ = text::confusable_with_own(raw, "scope");
        }
    }
}
