use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    Terrestrial,
    Submarine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkTo {
    pub code: &'static str,
    pub distance: f64,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // reference data; not every field is used
pub struct Country {
    pub code: &'static str,
    pub name: &'static str,
    pub continent: Option<&'static str>,
    pub other_names: Vec<&'static str>,
    pub cable_connections_number: usize,
    pub internet_exchanges_number: usize,
    pub links: Vec<LinkTo>,
}

impl PartialEq for Country {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
    }
}

impl Hash for Country {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.code.hash(state);
    }
}

impl Eq for Country {}

const COUNTRIES_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/countries.json"));

impl Country {
    pub fn from_str(s: &str) -> Option<&'static Country> {
        lazy_static! {
            static ref COUNTRIES: Vec<Country> = {
                serde_json::from_str(COUNTRIES_JSON).expect("failed to deserialize countries.json")
            };
            static ref COUNTRY_CODE_TO_COUNTRY: HashMap<String, usize> = {
                let mut result = HashMap::new();
                for (index, country) in COUNTRIES.iter().enumerate() {
                    result.insert(country.code.to_lowercase(), index);
                    result.insert(country.name.to_lowercase(), index);
                    for other in &country.other_names {
                        result.insert(other.to_lowercase(), index);
                    }
                }
                result
            };
        }
        let lowered = s.to_lowercase();
        COUNTRY_CODE_TO_COUNTRY
            .get(lowered.as_str())
            .map(|index| unsafe { COUNTRIES.get_unchecked(*index) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_country_lookup() {
        let us = Country::from_str("US").expect("US should exist");
        assert_eq!(us.code, "US");
        assert_eq!(us.name, "United States");

        let de = Country::from_str("germany").expect("germany should exist");
        assert_eq!(de.code, "DE");

        let ua = Country::from_str("ua").expect("ua should exist");
        assert_eq!(ua.code, "UA");
        assert_eq!(ua.name, "Ukraine");

        assert!(Country::from_str("nonexistent").is_none());
    }
}
