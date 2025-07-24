//! Modular NIP (Nostr Implementation Possibilities) support
//! 
//! This module provides a framework for implementing optional NIPs in a modular way.
//! NIP-01 is built into the core RelayHandler trait. This module handles optional NIPs.


// Optional NIPs (feature-gated)
#[cfg(feature = "nip11")]
pub mod nip11;

#[cfg(feature = "nip42")]
pub mod nip42;

#[cfg(feature = "nip45")]
pub mod nip45;

#[cfg(feature = "nip50")]
pub mod nip50;

/// Build the list of supported NIPs based on enabled features
pub fn build_supported_nips() -> Vec<u32> {
    let mut nips = vec![1]; // Always support NIP-01 (built into RelayHandler)
    
    #[cfg(feature = "nip11")]
    nips.push(11);
    
    #[cfg(feature = "nip42")]
    nips.push(42);
    
    #[cfg(feature = "nip45")]
    nips.push(45);
    
    #[cfg(feature = "nip50")]
    nips.push(50);
    
    nips
}

/// Helper to check if a specific NIP is supported
pub fn supports_nip(nip: u32) -> bool {
    match nip {
        1 => true, // NIP-01 is always supported
        #[cfg(feature = "nip11")]
        11 => true,
        #[cfg(feature = "nip42")]
        42 => true,
        #[cfg(feature = "nip45")]
        45 => true,
        #[cfg(feature = "nip50")]
        50 => true,
        _ => false,
    }
}