use serde::Deserialize;
use std::{collections::BTreeMap, env, fmt::Write, fs, path::Path};

#[derive(Deserialize)]
enum RawLinkType {
    Terrestrial,
    Submarine,
}

#[derive(Deserialize)]
struct RawLink {
    code: String,
    distance: f64,
    link_type: RawLinkType,
}

#[derive(Deserialize)]
struct RawCountry {
    code: String,
    name: String,
    other_names: Vec<String>,
    cable_connections_number: u16,
    internet_exchanges_number: u16,
    links: Vec<RawLink>,
}

fn format_country_code(code: &str, context: &str) -> String {
    assert!(
        code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase()),
        "invalid 2-letter uppercase ASCII country code {code:?} in {context}"
    );
    format!("CountryCode::new(*b\"{code}\")")
}

// Generates static country/link slices and a pre-sorted lowercase lookup table
// from src/countries.json so runtime lookups need no JSON parsing or heap setup.
fn main() {
    println!("cargo:rerun-if-changed=src/countries.json");
    let source =
        fs::read_to_string("src/countries.json").expect("failed to read src/countries.json");
    let countries: Vec<RawCountry> =
        serde_json::from_str(&source).expect("failed to parse src/countries.json");

    let mut lookup = BTreeMap::new();
    let mut out = String::with_capacity(source.len());
    out.push_str("static COUNTRIES: &[Country] = &[\n");

    for (index, c) in countries.iter().enumerate() {
        lookup.insert(c.code.to_lowercase(), index);
        lookup.insert(c.name.to_lowercase(), index);
        for other in &c.other_names {
            lookup.insert(other.to_lowercase(), index);
        }

        let country_code = format_country_code(&c.code, "country");
        write!(
            out,
            "    Country {{ code: {country_code}, name: {:?}, cable_connections_number: {}, internet_exchanges_number: {}, links: &[",
            c.name, c.cable_connections_number, c.internet_exchanges_number
        )
        .unwrap();
        for (i, link) in c.links.iter().enumerate() {
            let distance = link.distance as f32;
            assert!(
                link.distance.is_finite() && distance.is_finite() && distance > 0.0,
                "invalid f32 distance {:?} in link {} -> {}",
                link.distance,
                c.code,
                link.code
            );
            if i > 0 {
                out.push_str(", ");
            }
            let link_code = format_country_code(&link.code, &format!("link from {}", c.code));
            let link_type = match link.link_type {
                RawLinkType::Terrestrial => "Terrestrial",
                RawLinkType::Submarine => "Submarine",
            };
            write!(
                out,
                "LinkTo {{ code: {link_code}, distance: {distance:?}_f32, link_type: LinkType::{link_type} }}"
            )
            .unwrap();
        }
        out.push_str("] },\n");
    }

    out.push_str("];\n\nstatic COUNTRY_LOOKUP: &[(&str, usize)] = &[\n");
    for (key, index) in &lookup {
        writeln!(out, "    ({key:?}, {index}),").unwrap();
    }
    out.push_str("];\n");

    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("countries_data.rs");
    fs::write(dest, out).expect("failed to write countries_data.rs to OUT_DIR");
}
