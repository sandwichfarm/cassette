use cassette_tools::{Cassette, CassetteSchema, one_plus_one};
use wasm_bindgen::prelude::*;
use serde_json::json;

#[wasm_bindgen]
pub struct SandwichsFavs;

#[wasm_bindgen]
impl SandwichsFavs {
    #[wasm_bindgen]
    pub fn describe() -> String {
        <Self as Cassette>::describe()
    }
    
    #[wasm_bindgen]
    pub fn calculate() -> u64 {
        one_plus_one()
    }
    
    #[wasm_bindgen]
    pub fn get_schema() -> String {
        <Self as Cassette>::get_schema_json()
    }
}

impl Cassette for SandwichsFavs {
    fn describe() -> String {
        format!("Sandwich's Favorite Notes - Result: {}", one_plus_one())
    }
    
    fn get_schema() -> CassetteSchema {
        CassetteSchema {
            title: "Sandwich's Favorite Notes".to_string(),
            description: "A cassette for managing sandwich favorites".to_string(),
            schema_type: "object".to_string(),
            properties: json!({
                "name": {
                    "type": "string",
                    "description": "The name of the sandwich"
                },
                "ingredients": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "List of ingredients in the sandwich"
                },
                "rating": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 5,
                    "description": "Rating from 1-5 stars"
                }
            }),
            required: vec!["name".to_string(), "ingredients".to_string()],
        }
    }
} 