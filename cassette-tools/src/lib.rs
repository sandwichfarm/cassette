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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Value>,
}

impl Default for CassetteSchema {
    fn default() -> Self {
        Self {
            title: "Default Schema".to_string(),
            description: "Default schema description".to_string(),
            properties: json!({}),
            schema_type: "object".to_string(),
            required: vec![],
            items: None,
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

/// NIP-01 Implementation
pub mod nip01 {
    use super::*;

    /// Client Request message (REQ)
    pub struct ClientReq;
    
    impl Cassette for ClientReq {
        fn describe() -> String {
            "NIP-01 Client Request (REQ) message".to_string()
        }
        
        fn get_schema() -> CassetteSchema {
            CassetteSchema {
                title: "Client Request".to_string(),
                description: "A message sent by clients to request events from relays".to_string(),
                schema_type: "array".to_string(),
                properties: json!({}),
                required: vec![],
                items: Some(json!([
                    {
                        "const": "REQ"
                    },
                    {
                        "type": "string",
                        "description": "An identifier for the subscription, this should be unique."
                    },
                    {
                        "type": "object",
                        "properties": {
                            "ids": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "pattern": "^[a-f0-9]{64}$"
                                },
                                "description": "A list of event ids in lowercase hex"
                            },
                            "authors": {
                                "type": "array",
                                "items": {
                                    "type": "string",
                                    "pattern": "^[a-f0-9]{64}$"
                                },
                                "description": "A list of lowercase pubkeys, matching the pubkey of an event"
                            },
                            "kinds": {
                                "type": "array",
                                "items": {
                                    "type": "integer",
                                    "minimum": "0"
                                },
                                "description": "A list of kind numbers"
                            },
                            "since": {
                                "type": "integer",
                                "minimum": "0",
                                "description": "An integer Unix timestamp in seconds, where events must have created_at >= since"
                            },
                            "until": {
                                "type": "integer",
                                "minimum": "0",
                                "description": "An integer Unix timestamp in seconds, where events must have created_at <= until"
                            },
                            "limit": {
                                "type": "integer",
                                "minimum": "1",
                                "description": "The maximum number of events relays SHOULD return in the initial query"
                            }
                        },
                        "patternProperties": {
                            "^#[a-zA-Z]$": {
                                "type": "array",
                                "items": {
                                    "type": "string"
                                },
                                "description": "A list of tag values, where specific tags (#e, #p) have designated meanings"
                            }
                        },
                        "additionalProperties": false
                    }
                ])),
            }
        }
    }
    
    /// Relay Event message (EVENT)
    pub struct RelayEvent;
    
    impl Cassette for RelayEvent {
        fn describe() -> String {
            "NIP-01 Relay Event (EVENT) message".to_string()
        }
        
        fn get_schema() -> CassetteSchema {
            CassetteSchema {
                title: "Relay Event".to_string(),
                description: "A message sent by relays to clients in response to a client request.".to_string(),
                schema_type: "array".to_string(),
                properties: json!({}),
                required: vec![],
                items: Some(json!([
                    {
                        "const": "EVENT"
                    },
                    {
                        "type": "string",
                        "description": "The id of the subscription that the note is being sent in response to"
                    },
                    {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The content of the note"
                            },
                            "created_at": {
                                "type": "integer",
                                "description": "The timestamp of the note creation"
                            },
                            "id": {
                                "type": "string",
                                "pattern": "^[a-f0-9]{64}$",
                                "description": "The id is a hash derived as specified in NIP-01"
                            },
                            "kind": {
                                "type": "integer"
                            },
                            "pubkey": {
                                "type": "string",
                                "pattern": "^[a-f0-9]{64}$",
                                "description": "The public key of the note's author"
                            },
                            "sig": {
                                "type": "string",
                                "description": "The cryptographic signature of the note"
                            },
                            "tags": {
                                "type": "array",
                                "description": "The tags of the note",
                                "items": {
                                    "type": "array",
                                    "items": {
                                        "type": "string"
                                    }
                                }
                            }
                        },
                        "required": [
                            "content",
                            "created_at",
                            "id",
                            "kind",
                            "pubkey",
                            "sig",
                            "tags"
                        ]
                    }
                ])),
            }
        }
    }
    
    /// Relay Notice message (NOTICE)
    pub struct RelayNotice;
    
    impl Cassette for RelayNotice {
        fn describe() -> String {
            "NIP-01 Relay Notice (NOTICE) message".to_string()
        }
        
        fn get_schema() -> CassetteSchema {
            CassetteSchema {
                title: "Relay Notice".to_string(),
                description: "A message sent by relays to clients, usually to inform them of an issue.".to_string(),
                schema_type: "array".to_string(),
                properties: json!({}),
                required: vec![],
                items: Some(json!([
                    {
                        "const": "NOTICE"
                    },
                    {
                        "type": "string"
                    }
                ])),
            }
        }
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
