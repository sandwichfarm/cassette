use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

pub fn one_plus_one() -> u64 {
    1 + 1
}

/// JSON Schema for a Cassette
#[derive(Serialize, Deserialize)]
pub struct CassetteSchema {
    pub title: String,
    pub description: String,
    pub properties: Value,
    #[serde(rename = "type")]
    pub schema_type: String,
    pub required: Vec<String>,
}

impl Default for CassetteSchema {
    fn default() -> Self {
        Self {
            title: "Default Schema".to_string(),
            description: "Default schema description".to_string(),
            properties: json!({}),
            schema_type: "object".to_string(),
            required: vec![],
        }
    }
}

/// Trait that all cassettes must implement
pub trait Cassette {
    /// Returns a description of the cassette
    fn describe() -> String;
    
    /// Returns the JSON schema for the cassette
    fn get_schema() -> CassetteSchema;
    
    /// Generates the JSON schema string
    fn get_schema_json() -> String {
        let schema = Self::get_schema();
        serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
