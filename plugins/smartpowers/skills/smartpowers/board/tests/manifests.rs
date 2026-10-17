//! The plugin catalogs and manifests are valid JSON. The shell tests
//! (tests/test_marketplace.sh) compare their values but cannot parse JSON.

mod support;

use std::path::Path;

use support::Json;

#[test]
fn catalogs_and_manifests_parse() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../..");
    for file in [
        ".agents/plugins/marketplace.json",
        ".claude-plugin/marketplace.json",
        "plugins/smartpowers/.claude-plugin/plugin.json",
        "plugins/smartpowers/.codex-plugin/plugin.json",
    ] {
        let text = std::fs::read_to_string(repo.join(file)).unwrap();
        let json = Json::parse(&text).unwrap_or_else(|error| panic!("{file}: {error}"));
        assert_eq!(json["name"].str(), "smartpowers", "{file}");
    }
}

#[test]
fn parser_rejects_invalid_json() {
    for text in ["{", "{\"a\":1,}", "[1 2]", "\"\\x\"", "{} {}", "nul"] {
        assert!(Json::parse(text).is_err(), "accepted {text:?}");
    }
    let json = Json::parse(r#"{"a":[1,-2.5e3,true,null,"\u00e9\ud83d\ude00"]}"#).unwrap();
    assert_eq!(json["a"][4].str(), "é😀");
}
