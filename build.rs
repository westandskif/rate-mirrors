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
    continent: Option<String>,
    other_names: Vec<String>,
    cable_connections_number: usize,
    internet_exchanges_number: usize,
    links: Vec<RawLink>,
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

        write!(
            out,
            "    Country {{ code: {:?}, name: {:?}, continent: ",
            c.code, c.name
        )
        .unwrap();
        match &c.continent {
            Some(continent) => write!(out, "Some({continent:?})").unwrap(),
            None => out.push_str("None"),
        }
        out.push_str(", other_names: &[");
        for (i, other) in c.other_names.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write!(out, "{other:?}").unwrap();
        }
        write!(
            out,
            "], cable_connections_number: {}, internet_exchanges_number: {}, links: &[",
            c.cable_connections_number, c.internet_exchanges_number
        )
        .unwrap();
        for (i, link) in c.links.iter().enumerate() {
            assert!(
                link.distance.is_finite(),
                "non-finite distance in link {} -> {}",
                c.code,
                link.code
            );
            if i > 0 {
                out.push_str(", ");
            }
            let link_type = match link.link_type {
                RawLinkType::Terrestrial => "Terrestrial",
                RawLinkType::Submarine => "Submarine",
            };
            write!(
                out,
                "LinkTo {{ code: {:?}, distance: {:?}_f64, link_type: LinkType::{link_type} }}",
                link.code, link.distance
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
