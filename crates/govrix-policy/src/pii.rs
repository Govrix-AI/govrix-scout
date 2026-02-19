pub fn mask_pii(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_passthrough() {
        assert_eq!(mask_pii("hello@test.com"), "hello@test.com");
    }
}
