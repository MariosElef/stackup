use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn toggle<T: PartialEq>(list: &mut Vec<T>, item: T) {
    if let Some(i) = list.iter().position(|x| *x == item) {
        list.remove(i);
    } else {
        list.push(item);
    }
}

/// How well `query` matches `text`: `None` when it does not, else a score where lower is better.
/// Every character of the query must appear in the text, in order; the score is how far in the
/// first one is, plus the gaps between the rest, so a prefix or a whole word beats scattered
/// letters. Case and spaces in the query do not matter.
pub fn fuzzy(query: &str, text: &str) -> Option<u32> {
    let wanted: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let (mut from, mut last, mut score) = (0, None::<usize>, 0u32);
    for c in wanted {
        let at = from + text.get(from..)?.iter().position(|t| *t == c)?;
        score += match last {
            None => at as u32,
            Some(l) => (at - l - 1) as u32,
        };
        last = Some(at);
        from = at + 1;
    }
    Some(score)
}

/// The bytes a terminal would send for this key: what a program on the in-app terminal reads.
pub fn key_bytes(key: KeyEvent) -> Vec<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let mut out: Vec<u8> = match key.code {
        KeyCode::Char(c) if ctrl && c.is_ascii() => vec![(c.to_ascii_lowercase() as u8) & 0x1f],
        KeyCode::Char(_) if ctrl => vec![],
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\r".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        _ => vec![],
    };
    if alt && !out.is_empty() {
        out.insert(0, 0x1b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> Vec<u8> {
        key_bytes(KeyEvent::new(code, mods))
    }

    #[test]
    fn a_query_matches_when_its_letters_appear_in_order() {
        assert_eq!(
            fuzzy("", "anything"),
            Some(0),
            "nothing typed matches everything"
        );
        assert!(fuzzy("up", "Run up on shop").is_some());
        assert!(
            fuzzy("rus", "Run up on shop").is_some(),
            "scattered letters count"
        );
        assert!(fuzzy("pu", "up").is_none(), "but only in order");
        assert!(fuzzy("xyz", "Run up on shop").is_none());
        assert_eq!(
            fuzzy("UP S", "up shop"),
            fuzzy("ups", "up shop"),
            "case and spaces do not matter"
        );
    }

    #[test]
    fn closer_matches_score_better() {
        let prefix = fuzzy("sho", "shop").unwrap();
        let scattered = fuzzy("sho", "s-h-o-p").unwrap();
        let late = fuzzy("sho", "my shop").unwrap();
        assert!(
            prefix < scattered && prefix < late,
            "{prefix} {scattered} {late}"
        );
    }

    #[test]
    fn keys_become_what_a_terminal_sends() {
        assert_eq!(key(KeyCode::Char('a'), KeyModifiers::NONE), b"a");
        assert_eq!(key(KeyCode::Enter, KeyModifiers::NONE), b"\r");
        assert_eq!(key(KeyCode::Char('c'), KeyModifiers::CONTROL), [3]);
        assert_eq!(key(KeyCode::Up, KeyModifiers::NONE), b"\x1b[A");
        assert_eq!(key(KeyCode::Char('x'), KeyModifiers::ALT), b"\x1bx");
        assert_eq!(key(KeyCode::F(5), KeyModifiers::NONE), b"");
    }
}
