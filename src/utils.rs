/// Quote a PostgreSQL identifier by wrapping it in double quotes and doubling any
/// internal double quotes.  This matches the SQL standard and prevents injection
/// through schema, table, role, or function names.
pub(crate) fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_identifier() {
        assert_eq!(quote_identifier("foo"), "\"foo\"");
    }

    #[test]
    fn empty_string() {
        assert_eq!(quote_identifier(""), "\"\"");
    }

    #[test]
    fn single_embedded_double_quote() {
        // fo"o  →  "fo""o"
        assert_eq!(quote_identifier("fo\"o"), "\"fo\"\"o\"");
    }

    #[test]
    fn two_consecutive_embedded_double_quotes() {
        // a""b  →  "a""""b"
        assert_eq!(quote_identifier("a\"\"b"), "\"a\"\"\"\"b\"");
    }

    #[test]
    fn reserved_word_is_just_wrapped() {
        assert_eq!(quote_identifier("select"), "\"select\"");
    }

    #[test]
    fn identifier_with_spaces() {
        assert_eq!(quote_identifier("my schema"), "\"my schema\"");
    }
}
