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
