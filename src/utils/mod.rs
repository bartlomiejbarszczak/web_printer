#[macro_export]
macro_rules! capitalize {
    ($s:expr) => {{
        let input = $s.to_lowercase();

        let mut chars: Vec<char> = input.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect::<String>()
    }};
}