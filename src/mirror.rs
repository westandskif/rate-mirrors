use crate::countries::Country;
use std::fmt;
use url::Url;

#[derive(Debug)]
pub struct MirrorInfo {
    pub url: Url,
    pub country: Option<&'static Country>,
}

impl MirrorInfo {
    pub fn new(url: Url, country: Option<&str>) -> Self {
        let country = country.and_then(Country::from_str);
        Self { url, country }
    }

    pub fn parse(input: &str, sep: &str) -> Result<Self, MirrorParseError> {
        let args = input.trim().split(sep).collect::<Vec<_>>();

        match args.len() {
            1 => match Url::parse(args[0]) {
                Ok(url) => Ok(MirrorInfo::new(url, None)),
                Err(_) => Err(MirrorParseError::BadUrl(args[0].to_string())),
            },
            2 => match Url::parse(args[0]) {
                Ok(url) => Ok(MirrorInfo::new(url, Some(args[1]))),
                Err(_) => match Url::parse(args[1]) {
                    Ok(url) => Ok(MirrorInfo::new(url, Some(args[0]))),
                    Err(_) => Err(MirrorParseError::BadUrl(args[1].to_string())),
                },
            },
            _ => Err(MirrorParseError::BadLine(input.to_string())),
        }
    }
}

impl fmt::Display for MirrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.country {
            Some(country) => {
                write!(f, "[{}] {}", country.code, &self.url)
            }
            None => {
                write!(f, "{}", &self.url)
            }
        }
    }
}

#[derive(Debug)]
pub enum MirrorParseError {
    BadLine(String),
    BadUrl(String),
}

impl fmt::Display for MirrorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirrorParseError::BadLine(s) => write!(f, "Bad line: {}", s),
            MirrorParseError::BadUrl(s) => write!(f, "Bad url: {}", s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Mirror {
    pub url: Url,
    pub url_to_test: Url,
    pub country: Option<&'static Country>,
}

impl Mirror {
    pub fn with_test_path(
        url: Url,
        path_to_test: &str,
        country: Option<&'static Country>,
    ) -> Option<Self> {
        if url.cannot_be_a_base() {
            return None;
        }
        let url_to_test = url.join(path_to_test).ok()?;
        Some(Self {
            url,
            url_to_test,
            country,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_test_path_rejects_non_base_and_failed_joins() {
        for bad in ["mailto:admin@example.com", "data:text/plain,hello"] {
            let url = Url::parse(bad).unwrap();
            assert!(Mirror::with_test_path(url.clone(), "core.db", None).is_none());
            assert!(
                Mirror::with_test_path(url, "https://override.example/core.db", None).is_none()
            );
        }

        // Joining "http:" fails on https:// bases and succeeds on http:// bases in url 2.x.
        let https = Url::parse("https://mirror.example/repo/").unwrap();
        let http = Url::parse("http://mirror.example/repo/").unwrap();
        assert!(Mirror::with_test_path(https.clone(), "http:", None).is_none());
        assert!(Mirror::with_test_path(http, "http:", None).is_some());

        let country = Country::from_str("DE");
        let mirror = Mirror::with_test_path(https, "extra/extra.db", country).unwrap();
        assert_eq!(
            mirror.url_to_test.as_str(),
            "https://mirror.example/repo/extra/extra.db"
        );
        assert_eq!(mirror.country.map(|c| c.code), Some("DE"));
    }
}
