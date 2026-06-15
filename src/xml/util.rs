pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
    }
}

pub fn camel_case(s: &str) -> String {
    s.to_lowercase()
        .split('_')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join("")
}

pub fn lower_camel_case(s: &str) -> String {
    let lowered = s.to_lowercase();
    let parts: Vec<&str> = lowered.split('_').collect();
    if parts.len() == 1 {
        return parts[0].to_string();
    }

    let head = parts[0];
    let tail: String = parts[1..].iter().map(|part| capitalize(part)).collect();
    format!("{head}{tail}")
}
