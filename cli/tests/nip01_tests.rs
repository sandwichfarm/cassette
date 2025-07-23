// NIP-01 implementation tests
use serde::{Deserialize, Serialize};
use serde_json::json;

// Define the same Note struct as in the main code
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Note {
    id: String,
    pubkey: String,
    created_at: i64,
    kind: i64,
    tags: Vec<Vec<String>>,
    content: String,
    sig: String,
}

// Define the Filter struct as in the main code
#[derive(Debug, Deserialize)]
struct Filter {
    #[serde(default)]
    ids: Option<Vec<String>>,
    #[serde(default)]
    authors: Option<Vec<String>>,
    #[serde(default)]
    kinds: Option<Vec<i64>>,
    #[serde(default, flatten)]
    tag_filters: std::collections::HashMap<String, Vec<String>>,
    #[serde(default)]
    since: Option<i64>,
    #[serde(default)]
    until: Option<i64>,
    #[serde(default)]
    limit: Option<usize>,
}

// Implement the filter matching functionality directly in the test file
fn matches_filter(event: &Note, filter: &Filter) -> bool {
    // Check for ID match
    if let Some(ids) = &filter.ids {
        if !ids.iter().any(|id| id == &event.id) {
            return false;
        }
    }

    // Check for author match
    if let Some(authors) = &filter.authors {
        if !authors.iter().any(|author| author == &event.pubkey) {
            return false;
        }
    }

    // Check for kind match
    if let Some(kinds) = &filter.kinds {
        if !kinds.iter().any(|kind| *kind == event.kind) {
            return false;
        }
    }

    // Check for time range match
    if let Some(since) = filter.since {
        if event.created_at < since {
            return false;
        }
    }

    if let Some(until) = filter.until {
        if event.created_at > until {
            return false;
        }
    }

    // Check for tag filters
    for (key, values) in &filter.tag_filters {
        if key.starts_with('&') && key.len() == 2 {
            // NIP-1.9 tag filter: All values must match
            let tag_name = &key[1..];
            let tag_values: Vec<_> = event.tags.iter()
                .filter(|t| t.get(0).map_or(false, |n| n == tag_name))
                .filter_map(|t| t.get(1))
                .collect();

            if !values.iter().all(|v| tag_values.contains(&v)) {
                return false;
            }
        } else if key.starts_with('#') && key.len() == 2 {
            // Regular tag filter: Any value must match
            let tag_name = &key[1..];
            let tag_values: Vec<_> = event.tags.iter()
                .filter(|t| t.get(0).map_or(false, |n| n == tag_name))
                .filter_map(|t| t.get(1))
                .collect();

            if !values.iter().any(|v| tag_values.contains(&v)) {
                return false;
            }
        }
    }

    true
}

// Helper function to create a test note
fn create_test_note() -> Note {
    Note {
        id: "test_id".to_string(),
        pubkey: "test_pubkey".to_string(),
        created_at: 1234567890,
        kind: 1,
        tags: vec![
            vec!["t".to_string(), "tag1".to_string()],
            vec!["t".to_string(), "tag2".to_string()],
            vec!["p".to_string(), "pubkey1".to_string()],
        ],
        content: "test content".to_string(),
        sig: "test_sig".to_string(),
    }
}

#[test]
fn test_id_filter() {
    let note = create_test_note();
    
    // Test with matching ID
    let filter = Filter {
        ids: Some(vec!["test_id".to_string()]),
        authors: None,
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: None,
        limit: None,
    };
    assert!(matches_filter(&note, &filter), "Note should match filter with matching ID");
    
    // Test with non-matching ID
    let filter = Filter {
        ids: Some(vec!["wrong_id".to_string()]),
        authors: None,
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: None,
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter with non-matching ID");
}

#[test]
fn test_author_filter() {
    let note = create_test_note();
    
    // Test with matching author
    let filter = Filter {
        ids: None,
        authors: Some(vec!["test_pubkey".to_string()]),
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: None,
        limit: None,
    };
    assert!(matches_filter(&note, &filter), "Note should match filter with matching author");
    
    // Test with non-matching author
    let filter = Filter {
        ids: None,
        authors: Some(vec!["wrong_pubkey".to_string()]),
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: None,
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter with non-matching author");
}

#[test]
fn test_kind_filter() {
    let note = create_test_note();
    
    // Test with matching kind
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: Some(vec![1]),
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: None,
        limit: None,
    };
    assert!(matches_filter(&note, &filter), "Note should match filter with matching kind");
    
    // Test with non-matching kind
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: Some(vec![2]),
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: None,
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter with non-matching kind");
}

#[test]
fn test_timestamp_filters() {
    let note = create_test_note();
    
    // Test with matching time range
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: Some(1234567889),
        until: Some(1234567891),
        limit: None,
    };
    assert!(matches_filter(&note, &filter), "Note should match filter with matching time range");
    
    // Test with out-of-range timestamp (too early)
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: Some(1234567891),
        until: None,
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter with timestamp before since");
    
    // Test with out-of-range timestamp (too late)
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters: std::collections::HashMap::new(),
        since: None,
        until: Some(1234567889),
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter with timestamp after until");
}

#[test]
fn test_tag_filters() {
    let note = create_test_note();
    
    // Test with matching tag
    let mut tag_filters = std::collections::HashMap::new();
    tag_filters.insert("#t".to_string(), vec!["tag1".to_string()]);
    
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters,
        since: None,
        until: None,
        limit: None,
    };
    assert!(matches_filter(&note, &filter), "Note should match filter with matching tag");
    
    // Test with non-matching tag
    let mut tag_filters = std::collections::HashMap::new();
    tag_filters.insert("#t".to_string(), vec!["nonexistent".to_string()]);
    
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters,
        since: None,
        until: None,
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter with non-matching tag");
    
    // Test with NIP-1.9 tag filter (all values must match)
    let mut tag_filters = std::collections::HashMap::new();
    tag_filters.insert("&t".to_string(), vec!["tag1".to_string(), "tag2".to_string()]);
    
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters,
        since: None,
        until: None,
        limit: None,
    };
    assert!(matches_filter(&note, &filter), "Note should match filter with all specified tag values");
    
    // Test with NIP-1.9 tag filter (not all values match)
    let mut tag_filters = std::collections::HashMap::new();
    tag_filters.insert("&t".to_string(), vec!["tag1".to_string(), "nonexistent".to_string()]);
    
    let filter = Filter {
        ids: None,
        authors: None,
        kinds: None,
        tag_filters,
        since: None,
        until: None,
        limit: None,
    };
    assert!(!matches_filter(&note, &filter), "Note should not match filter when not all tag values match");
}

#[test]
fn test_req_json_parsing() {
    // Test parsing a valid REQ message
    let req_json = json!([
        "REQ",
        "subscription_id",
        {
            "kinds": [1, 2],
            "authors": ["pubkey1", "pubkey2"],
            "since": 1234567890,
            "until": 1234569999,
            "limit": 5,
            "#e": ["event1", "event2"],
            "#p": ["pubkey1", "pubkey2"]
        }
    ]).to_string();
    
    let parsed: serde_json::Value = serde_json::from_str(&req_json).unwrap();
    
    assert_eq!(parsed[0], "REQ");
    assert_eq!(parsed[1], "subscription_id");
    assert_eq!(parsed[2]["kinds"][0], 1);
    assert_eq!(parsed[2]["kinds"][1], 2);
    assert_eq!(parsed[2]["authors"][0], "pubkey1");
    assert_eq!(parsed[2]["since"], 1234567890);
    assert_eq!(parsed[2]["until"], 1234569999);
    assert_eq!(parsed[2]["limit"], 5);
    assert_eq!(parsed[2]["#e"][0], "event1");
    assert_eq!(parsed[2]["#p"][0], "pubkey1");
}

#[test]
fn test_close_json_parsing() {
    // Test parsing a valid CLOSE message
    let close_json = json!([
        "CLOSE",
        "subscription_id"
    ]).to_string();
    
    let parsed: serde_json::Value = serde_json::from_str(&close_json).unwrap();
    
    assert_eq!(parsed[0], "CLOSE");
    assert_eq!(parsed[1], "subscription_id");
} 