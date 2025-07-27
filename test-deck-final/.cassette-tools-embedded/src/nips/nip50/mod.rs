//! NIP-50: Search Capability
//! 
//! Provides search functionality for events using text queries

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Search filter extensions as defined by NIP-50
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchExtensions {
    /// Include spam in search results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_spam: Option<bool>,
    
    /// Filter by NIP-05 domain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    
    /// Filter by language (ISO code)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    
    /// Filter by sentiment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment: Option<String>, // "negative", "neutral", "positive"
    
    /// Include/exclude NSFW content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
}

/// Parsed search query with extensions
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Main search terms
    pub terms: Vec<String>,
    /// Search extensions
    pub extensions: SearchExtensions,
}

/// Simple search scoring for events
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The event that matches
    pub event: Value,
    /// Search score (higher = better match)
    pub score: f32,
}

/// Parse a search query string into terms and extensions
pub fn parse_search_query(query: &str) -> SearchQuery {
    let mut terms = Vec::new();
    let mut extensions = SearchExtensions {
        include_spam: None,
        domain: None,
        language: None,
        sentiment: None,
        nsfw: None,
    };
    
    // Split query into words and handle extensions
    for word in query.split_whitespace() {
        if word.starts_with("include:") {
            if word == "include:spam" {
                extensions.include_spam = Some(true);
            }
        } else if word.starts_with("domain:") {
            if let Some(domain) = word.strip_prefix("domain:") {
                extensions.domain = Some(domain.to_string());
            }
        } else if word.starts_with("language:") {
            if let Some(lang) = word.strip_prefix("language:") {
                extensions.language = Some(lang.to_string());
            }
        } else if word.starts_with("sentiment:") {
            if let Some(sentiment) = word.strip_prefix("sentiment:") {
                if ["negative", "neutral", "positive"].contains(&sentiment) {
                    extensions.sentiment = Some(sentiment.to_string());
                }
            }
        } else if word.starts_with("nsfw:") {
            if let Some(nsfw_str) = word.strip_prefix("nsfw:") {
                extensions.nsfw = nsfw_str.parse().ok();
            }
        } else {
            // Regular search term
            terms.push(word.to_lowercase());
        }
    }
    
    SearchQuery { terms, extensions }
}

/// Score an event against search terms
pub fn score_event(event: &Value, query: &SearchQuery) -> f32 {
    let mut score = 0.0;
    
    if query.terms.is_empty() {
        return 0.0;
    }
    
    // Get event content for searching
    let content = event.get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_lowercase();
    
    // Simple scoring: count term matches in content
    for term in &query.terms {
        let matches = content.matches(term).count() as f32;
        score += matches;
        
        // Bonus for exact phrase matches
        if content.contains(term) {
            score += 0.5;
        }
        
        // Bonus for matches at word boundaries
        let words: Vec<&str> = content.split_whitespace().collect();
        for word in words {
            if word == term {
                score += 1.0;
            } else if word.contains(term) {
                score += 0.3;
            }
        }
    }
    
    // Apply extension filters (return 0 if doesn't match)
    if let Some(ref domain) = query.extensions.domain {
        // Check NIP-05 domain - would need additional metadata
        // For now, skip events that don't match (simplified)
        if !content.contains(domain) {
            return 0.0;
        }
    }
    
    // Language filtering would require language detection
    // For now, we'll skip this advanced feature
    
    // Sentiment filtering would require sentiment analysis
    // For now, we'll skip this advanced feature
    
    // NSFW filtering would require content classification
    // For now, we'll skip this advanced feature
    
    score
}

/// Search through events and return sorted results
pub fn search_events(events: &[Value], query_str: &str, limit: Option<usize>) -> Vec<Value> {
    let query = parse_search_query(query_str);
    let mut results: Vec<SearchResult> = Vec::new();
    
    // Score all events
    for event in events {
        let score = score_event(event, &query);
        if score > 0.0 {
            results.push(SearchResult {
                event: event.clone(),
                score,
            });
        }
    }
    
    // Sort by score (descending)
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    
    // Apply limit and return events
    let limited_results = if let Some(limit) = limit {
        results.into_iter().take(limit).collect()
    } else {
        results
    };
    
    limited_results.into_iter().map(|r| r.event).collect()
}

/// Handle search request for NIP-50
pub fn handle_search(events: &[Value], filters: &[Value]) -> String {
    if filters.is_empty() {
        return json!(["NOTICE", "Search requires filter parameters"]).to_string();
    }
    
    let mut all_results = Vec::new();
    
    for filter in filters {
        if let Some(filter_obj) = filter.as_object() {
            // Check if this filter has a search field
            if let Some(search_query) = filter_obj.get("search").and_then(|s| s.as_str()) {
                // Get limit if specified
                let limit = filter_obj.get("limit").and_then(|l| l.as_u64()).map(|l| l as usize);
                
                // Filter events by other criteria first (kinds, authors, etc.)
                let mut filtered_events = events.to_vec();
                
                // Apply kinds filter
                if let Some(kinds) = filter_obj.get("kinds").and_then(|k| k.as_array()) {
                    let kind_numbers: Vec<u64> = kinds.iter()
                        .filter_map(|k| k.as_u64())
                        .collect();
                    filtered_events.retain(|event| {
                        if let Some(kind) = event.get("kind").and_then(|k| k.as_u64()) {
                            kind_numbers.contains(&kind)
                        } else {
                            false
                        }
                    });
                }
                
                // Apply authors filter
                if let Some(authors) = filter_obj.get("authors").and_then(|a| a.as_array()) {
                    let author_pubkeys: Vec<String> = authors.iter()
                        .filter_map(|a| a.as_str())
                        .map(|s| s.to_string())
                        .collect();
                    filtered_events.retain(|event| {
                        if let Some(pubkey) = event.get("pubkey").and_then(|p| p.as_str()) {
                            author_pubkeys.iter().any(|author| pubkey.starts_with(author))
                        } else {
                            false
                        }
                    });
                }
                
                // Apply time filters
                if let Some(since) = filter_obj.get("since").and_then(|s| s.as_u64()) {
                    filtered_events.retain(|event| {
                        if let Some(created_at) = event.get("created_at").and_then(|t| t.as_u64()) {
                            created_at >= since
                        } else {
                            false
                        }
                    });
                }
                
                if let Some(until) = filter_obj.get("until").and_then(|u| u.as_u64()) {
                    filtered_events.retain(|event| {
                        if let Some(created_at) = event.get("created_at").and_then(|t| t.as_u64()) {
                            created_at <= until
                        } else {
                            false
                        }
                    });
                }
                
                // Perform search on filtered events
                let search_results = search_events(&filtered_events, search_query, limit);
                all_results.extend(search_results);
            }
        }
    }
    
    // Remove duplicates (in case multiple filters matched the same event)
    let mut seen_ids = std::collections::HashSet::new();
    all_results.retain(|event| {
        if let Some(id) = event.get("id").and_then(|i| i.as_str()) {
            seen_ids.insert(id.to_string())
        } else {
            true // Keep events without IDs
        }
    });
    
    // Format results as JSON string for return
    let events_json: Vec<String> = all_results.iter()
        .map(|event| event.to_string())
        .collect();
    
    events_json.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    #[test]
    fn test_parse_search_query() {
        let query = parse_search_query("bitcoin lightning domain:example.com");
        assert_eq!(query.terms, vec!["bitcoin", "lightning"]);
        assert_eq!(query.extensions.domain, Some("example.com".to_string()));
    }
    
    #[test]
    fn test_score_event() {
        let event = json!({
            "id": "test",
            "content": "I love Bitcoin and Lightning Network",
            "kind": 1,
            "pubkey": "test"
        });
        
        let query = parse_search_query("bitcoin lightning");
        let score = score_event(&event, &query);
        assert!(score > 0.0);
    }
    
    #[test]
    fn test_search_events() {
        let events = vec![
            json!({
                "id": "1",
                "content": "Bitcoin is great",
                "kind": 1,
                "pubkey": "test"
            }),
            json!({
                "id": "2", 
                "content": "I prefer Ethereum",
                "kind": 1,
                "pubkey": "test"
            })
        ];
        
        let results = search_events(&events, "bitcoin", Some(10));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["id"], "1");
    }
}