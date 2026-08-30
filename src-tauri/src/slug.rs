use std::path::Path;

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;

    for ch in input.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch.is_whitespace() || matches!(ch, '-' | '_' | '.' | '/' | '\\') {
            Some('_')
        } else {
            None
        };

        match mapped {
            Some('_') if prev_underscore || out.is_empty() => {}
            Some('_') => {
                out.push('_');
                prev_underscore = true;
            }
            Some(c) => {
                out.push(c);
                prev_underscore = false;
            }
            None => {}
        }
    }

    while out.ends_with('_') {
        out.pop();
    }

    if out.is_empty() {
        "album".into()
    } else {
        out
    }
}

pub fn title_from_filename(path: &str) -> String {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Track");

    let mut out = String::new();
    let mut prev_space = false;
    for ch in stem.chars() {
        if ch == '_' || ch == '-' || ch.is_whitespace() {
            if !out.is_empty() && !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    let out = out.trim().to_string();
    if out.is_empty() {
        "Track".into()
    } else {
        out
    }
}

pub fn sanitize_filename_stem(input: &str) -> String {
    slugify(input)
}

pub fn unique_filename(stem: &str, ext: &str, used: &mut Vec<String>) -> String {
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    let mut candidate = format!("{stem}.{ext}");
    let mut n = 2;
    while used.iter().any(|e| e.eq_ignore_ascii_case(&candidate)) {
        candidate = format!("{stem}_{n}.{ext}");
        n += 1;
    }
    used.push(candidate.clone());
    candidate
}

pub fn lua_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 2);
    out.push('"');
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_album_names() {
        assert_eq!(slugify("Paranoid"), "paranoid");
        assert_eq!(slugify("Demon Days"), "demon_days");
        assert_eq!(slugify("  The Bends!! "), "the_bends");
        assert_eq!(slugify("***"), "album");
    }

    #[test]
    fn titles_from_files() {
        assert_eq!(
            title_from_filename("war_pigs_luke_s_wall_2012_remaster.mp3"),
            "war pigs luke s wall 2012 remaster"
        );
        assert_eq!(title_from_filename(r"C:\music\Iron Man.mp3"), "Iron Man");
    }

    #[test]
    fn unique_names_avoid_collisions() {
        let mut used = Vec::new();
        assert_eq!(unique_filename("track", "mp3", &mut used), "track.mp3");
        assert_eq!(unique_filename("track", "mp3", &mut used), "track_2.mp3");
    }

    #[test]
    fn lua_escapes_quotes() {
        assert_eq!(lua_escape(r#"Say "hello""#), r#""Say \"hello\"""#);
    }
}
