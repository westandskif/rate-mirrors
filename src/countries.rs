use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CountryCode([u8; 2]);

impl CountryCode {
    pub const fn new(bytes: [u8; 2]) -> Self {
        assert!(
            bytes[0].is_ascii_uppercase() && bytes[1].is_ascii_uppercase(),
            "CountryCode must be two uppercase ASCII letters"
        );
        Self(bytes)
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("CountryCode is valid ASCII")
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for CountryCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for CountryCode {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for CountryCode {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Terrestrial,
    Submarine,
}

#[derive(Debug, Clone, Copy)]
pub struct LinkTo {
    pub code: CountryCode,
    pub distance: f32,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // name is reference metadata verified in tests
pub struct Country {
    pub code: CountryCode,
    pub name: &'static str,
    pub cable_connections_number: u16,
    pub internet_exchanges_number: u16,
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
    use std::collections::HashSet;

    #[test]
    fn test_country_code_newtype() {
        let us = CountryCode::new(*b"US");
        let de = CountryCode::new(*b"DE");

        assert_eq!(us.as_str(), "US");
        assert_eq!(us.as_ref(), "US");
        assert_eq!(format!("{us}"), "US");
        assert_eq!(us, "US");
        assert_eq!(us, *"US");
        assert_ne!(us, "DE");
        assert!(de < us);

        let mut set = HashSet::new();
        assert!(set.insert(us));
        assert!(!set.insert(CountryCode::new(*b"US")));
        assert!(set.contains(&us));
        assert!(!set.contains(&de));

        assert_eq!(std::mem::size_of::<CountryCode>(), 2);
        assert_eq!(std::mem::size_of::<LinkTo>(), 8);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(std::mem::size_of::<Country>(), 40);
        #[cfg(target_pointer_width = "32")]
        assert_eq!(std::mem::size_of::<Country>(), 24);
    }

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
        assert_eq!(Country::from_str("GB").map(|c| c.code.as_str()), Some("UK"));
        assert_eq!(
            Country::from_str("Bosnia-Herzegovina").map(|c| c.code.as_str()),
            Some("BA")
        );
        assert_eq!(
            Country::from_str("virgin islands (u.s.)").map(|c| c.code.as_str()),
            Some("VI")
        );

        assert_eq!(
            Country::from_str("Curaçao").map(|c| c.code.as_str()),
            Some("CW")
        );
        assert_eq!(
            Country::from_str("CURAÇAO").map(|c| c.code.as_str()),
            Some("CW")
        );
        assert_eq!(
            Country::from_str("Côte d'Ivoire").map(|c| c.code.as_str()),
            Some("CI")
        );
        assert_eq!(
            Country::from_str("côte d'ivoire").map(|c| c.code.as_str()),
            Some("CI")
        );
        assert_eq!(
            Country::from_str("RÉUNION").map(|c| c.code.as_str()),
            Some("RE")
        );
        assert_eq!(
            Country::from_str("SAINT BARTHÉLEMY").map(|c| c.code.as_str()),
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
            assert_eq!(Country::from_str(country.code.as_str()), Some(country));
            assert_eq!(Country::from_str(country.name), Some(country));
            for link in country.links {
                assert!(
                    Country::from_str(link.code.as_str()).is_some(),
                    "unresolved link target {} from {}",
                    link.code,
                    country.code
                );
            }
        }
    }
}
