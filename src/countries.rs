use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Terrestrial,
    Submarine,
}

#[derive(Debug, Clone, Copy)]
pub struct LinkTo {
    pub code: &'static str,
    pub distance: f64,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // reference data; not every field is used
pub struct Country {
    pub code: &'static str,
    pub name: &'static str,
    pub continent: Option<&'static str>,
    pub other_names: &'static [&'static str],
    pub cable_connections_number: usize,
    pub internet_exchanges_number: usize,
    pub links: &'static [LinkTo],
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

include!(concat!(env!("OUT_DIR"), "/countries_data.rs"));

impl Country {
    pub fn from_str(s: &str) -> Option<&'static Country> {
        let lowered = s.to_lowercase();
        COUNTRY_LOOKUP
            .binary_search_by_key(&lowered.as_str(), |&(k, _)| k)
            .ok()
            .map(|i| &COUNTRIES[COUNTRY_LOOKUP[i].1])
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

    #[test]
    fn test_aliases_and_unicode_case_folding() {
        assert_eq!(Country::from_str("GB").map(|c| c.code), Some("UK"));
        assert_eq!(
            Country::from_str("Bosnia-Herzegovina").map(|c| c.code),
            Some("BA")
        );
        assert_eq!(
            Country::from_str("virgin islands (u.s.)").map(|c| c.code),
            Some("VI")
        );

        assert_eq!(Country::from_str("Curaçao").map(|c| c.code), Some("CW"));
        assert_eq!(Country::from_str("CURAÇAO").map(|c| c.code), Some("CW"));
        assert_eq!(
            Country::from_str("Côte d'Ivoire").map(|c| c.code),
            Some("CI")
        );
        assert_eq!(
            Country::from_str("côte d'ivoire").map(|c| c.code),
            Some("CI")
        );
        assert_eq!(Country::from_str("RÉUNION").map(|c| c.code), Some("RE"));
        assert_eq!(
            Country::from_str("SAINT BARTHÉLEMY").map(|c| c.code),
            Some("BL")
        );
    }

    #[test]
    fn test_country_table_invariants() {
        assert!(!COUNTRIES.is_empty());
        assert!(!COUNTRY_LOOKUP.is_empty());

        for pair in COUNTRY_LOOKUP.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "COUNTRY_LOOKUP is not strictly sorted: {:?} >= {:?}",
                pair[0].0,
                pair[1].0
            );
        }
        for &(_, index) in COUNTRY_LOOKUP {
            assert!(index < COUNTRIES.len());
        }

        for country in COUNTRIES {
            assert_eq!(Country::from_str(country.code), Some(country));
            assert_eq!(Country::from_str(country.name), Some(country));
            for other in country.other_names {
                assert_eq!(Country::from_str(other), Some(country));
            }
            for link in country.links {
                assert!(
                    Country::from_str(link.code).is_some(),
                    "unresolved link target {} from {}",
                    link.code,
                    country.code
                );
            }
        }
    }
}
