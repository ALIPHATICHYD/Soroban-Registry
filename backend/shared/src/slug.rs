//! Slug generation utilities for contracts and other entities.

/// Generates a URL-friendly slug from a string.
/// - Converts to lowercase
/// - Replaces non-alphanumeric characters with hyphens
/// - Trims leading/trailing hyphens
/// - Collapses multiple hyphens into one
pub fn slugify(s: &str) -> String {
    let mut slug = String::with_capacity(s.len());
    let mut last_was_hyphen = true; // Start with true to skip leading hyphens

    for c in s.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    // Trim trailing hyphen
    if slug.ends_with('-') {
        slug.pop();
    }

    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Soroban Contract V1"), "soroban-contract-v1");
        assert_eq!(slugify("MyContract@123"), "mycontract-123");
        assert_eq!(slugify("---Hey---"), "hey");
        assert_eq!(slugify("Multiple   Spaces"), "multiple-spaces");
        assert_eq!(slugify("Already-slugified"), "already-slugified");
        assert_eq!(slugify("Special!@#$%^&*Chars"), "special-chars");
    }
}
