//! NIP-11: Relay Information Document
//! 
//! Provides relay metadata and capability discovery through supported_nips

use serde::{Deserialize, Serialize};

/// Relay information structure as defined by NIP-11
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelayInfo {
    /// Relay name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Relay description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Administrative contact pubkey
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubkey: Option<String>,
    
    /// Administrative contact info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    
    /// List of supported NIPs
    pub supported_nips: Vec<u32>,
    
    /// Software identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub software: Option<String>,
    
    /// Software version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    
    /// Relay limitations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limitation: Option<RelayLimitation>,
    
    /// Relay retention policies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<Vec<RetentionPolicy>>,
    
    /// Relay-specific URLs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_urls: Option<Vec<String>>,
    
    /// Payment-related info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payments_info: Option<PaymentsInfo>,
    
    /// Relay icon URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Relay limitation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayLimitation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_message_length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_subscriptions: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_filters: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_limit: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_subid_length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_event_tags: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_content_length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_pow_difficulty: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_required: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_required: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_lower_limit: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_upper_limit: Option<i64>,
}

/// Event retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<u32>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

/// Payment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<PaymentAmount>,
}

/// Payment amount structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAmount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admission: Option<Vec<PaymentOption>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<Vec<PaymentOption>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication: Option<Vec<PaymentOption>>,
}

/// Individual payment option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentOption {
    pub amount: u64,
    pub unit: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<u32>,
}

/// Static relay info storage for WASM cassettes
static mut RELAY_INFO_JSON: Option<String> = None;

/// Initialize relay info from JSON string (called by CLI)
#[no_mangle]
pub extern "C" fn set_relay_info(json_ptr: *const u8, json_len: usize) -> i32 {
    unsafe {
        if json_ptr.is_null() || json_len == 0 {
            return -1;
        }
        
        let slice = std::slice::from_raw_parts(json_ptr, json_len);
        match std::str::from_utf8(slice) {
            Ok(json_str) => {
                // Validate the JSON
                match serde_json::from_str::<RelayInfo>(json_str) {
                    Ok(_) => {
                        RELAY_INFO_JSON = Some(json_str.to_string());
                        0
                    }
                    Err(_) => -2
                }
            }
            Err(_) => -3
        }
    }
}

impl Default for RelayLimitation {
    fn default() -> Self {
        Self {
            max_message_length: Some(16384),
            max_subscriptions: Some(20),
            max_filters: Some(10),
            max_limit: Some(1000),
            max_subid_length: Some(100),
            max_event_tags: Some(100),
            max_content_length: Some(65536),
            min_pow_difficulty: None,
            auth_required: Some(false),
            payment_required: Some(false),
            created_at_lower_limit: None,
            created_at_upper_limit: None,
        }
    }
}

/// Builder for RelayInfo to support CLI arguments
#[derive(Debug, Clone, Default)]
pub struct RelayInfoBuilder {
    pub name: Option<String>,
    pub description: Option<String>,
    pub pubkey: Option<String>,
    pub contact: Option<String>,
    pub supported_nips: Vec<u32>,
    pub software: Option<String>,
    pub version: Option<String>,
}

impl RelayInfoBuilder {
    pub fn new() -> Self {
        Self {
            supported_nips: crate::nips::build_supported_nips(),
            software: Some("cassette".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            ..Default::default()
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_pubkey(mut self, pubkey: String) -> Self {
        self.pubkey = Some(pubkey);
        self
    }

    pub fn with_contact(mut self, contact: String) -> Self {
        self.contact = Some(contact);
        self
    }

    pub fn build(self) -> RelayInfo {
        RelayInfo {
            name: self.name,
            description: self.description,
            pubkey: self.pubkey,
            contact: self.contact,
            supported_nips: self.supported_nips,
            software: self.software,
            version: self.version,
            limitation: Some(RelayLimitation::default()),
            retention: None,
            relay_urls: None,
            payments_info: None,
            icon: None,
        }
    }
}

/// Export function for WASM cassettes to provide relay information
#[cfg(feature = "nip11")]
#[no_mangle]
pub extern "C" fn info() -> *mut u8 {
    unsafe {
        match &RELAY_INFO_JSON {
            Some(json) => crate::string_to_ptr(json.clone()),
            None => {
                // Return minimal info with just supported NIPs
                let minimal_info = RelayInfo {
                    supported_nips: crate::nips::build_supported_nips(),
                    ..Default::default()
                };
                let json_str = serde_json::to_string(&minimal_info).unwrap_or_else(|_| "{}".to_string());
                crate::string_to_ptr(json_str)
            }
        }
    }
}