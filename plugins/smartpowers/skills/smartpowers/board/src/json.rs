//! Minimal JSON writer; the board only ever serializes, never parses.

pub enum Json {
    Null,
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(&'static str, Json)>),
}

impl Json {
    pub fn str(value: impl Into<String>) -> Json {
        Json::Str(value.into())
    }

    pub fn opt_str(value: Option<&str>) -> Json {
        value.map_or(Json::Null, Json::str)
    }

    pub fn strings<S: AsRef<str>>(items: &[S]) -> Json {
        Json::Arr(items.iter().map(|item| Json::str(item.as_ref())).collect())
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Str(s) => write_str(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_str(value: &str, out: &mut String) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Keep "</script>"-style sequences inert if the JSON is ever inlined.
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_nested_values_with_escaping() {
        let value = Json::Obj(vec![
            ("text", Json::str("a\"b\\c\n</x>\u{1}")),
            ("list", Json::Arr(vec![Json::Num(1), Json::Null])),
        ]);
        assert_eq!(
            value.render(),
            r#"{"text":"a\"b\\c\n\u003c/x>\u0001","list":[1,null]}"#
        );
    }
}
