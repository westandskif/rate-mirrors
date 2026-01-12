use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::read::GzDecoder;
use reqwest::Client;
use tar::Archive;
use tokio::time::timeout;
use url::Url;
use zstd::stream::read::Decoder as ZstdDecoder;

#[derive(Debug, Clone)]
pub struct FreshnessCheckResult {
    pub score: f64,
    pub packages_compared: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackageBuildDates {
    pub packages: HashMap<String, i64>,
}

pub async fn check_mirror(
    mirror_url: Url,
    base_path: &str,
    ref_local_dir: &str,
    timeout_ms: u64,
) -> FreshnessCheckResult {
    let db_url: Url = match mirror_url.join(&format!("{}.db", base_path)) {
        Ok(u) => u,
        Err(e) => {
            return FreshnessCheckResult {
                score: 0.0,
                packages_compared: 0,
                error: Some(format!("failed to build db url: {}", e)),
            }
        }
    };

    let db_filename = match Path::new(base_path).file_name() {
        Some(name) => name.to_string_lossy().to_string() + ".db",
        None => "mirror.db".to_string(),
    };

    // Load reference DB from local directory
    let reference = match load_reference_db(ref_local_dir, &db_filename) {
        Ok(p) => p,
        Err(e) => {
            return FreshnessCheckResult {
                score: 0.0,
                packages_compared: 0,
                error: Some(format!("ref db error: {}", e)),
            };
        }
    };

    // Download mirror DB
    let client = Client::new();
    let fetch = async {
        let resp = client
            .get(db_url.clone())
            .timeout(Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok::<_, String>(bytes.to_vec())
    };

    let mirror_bytes = match timeout(Duration::from_millis(timeout_ms), fetch).await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            return FreshnessCheckResult {
                score: 0.0,
                packages_compared: 0,
                error: Some(format!("download error: {}", e)),
            };
        }
        Err(_) => {
            return FreshnessCheckResult {
                score: 0.0,
                packages_compared: 0,
                error: Some("download timeout".to_string()),
            };
        }
    };

    let mirror_pkgs = match parse_db_bytes(&mirror_bytes) {
        Ok(p) => p,
        Err(e) => {
            return FreshnessCheckResult {
                score: 0.0,
                packages_compared: 0,
                error: Some(format!("parse error: {}", e)),
            };
        }
    };

    let (score, compared) = calculate_freshness_score(&mirror_pkgs, &reference);

    FreshnessCheckResult {
        score,
        packages_compared: compared,
        error: None,
    }
}

pub fn load_reference_db(dir: &str, db_filename: &str) -> Result<PackageBuildDates, String> {
    let path: PathBuf = Path::new(dir).join(db_filename);
    let data = std::fs::read(&path).map_err(|e| format!("read ref db {}: {}", path.display(), e))?;
    parse_db_bytes(&data)
}

fn parse_db_bytes(data: &[u8]) -> Result<PackageBuildDates, String> {
    // Try ZSTD
    if let Ok(mut decoder) = ZstdDecoder::new(data) {
        let mut buf = Vec::new();
        if decoder.read_to_end(&mut buf).is_ok() {
            if let Ok(pkgs) = parse_tar(&buf) {
                return Ok(pkgs);
            }
        }
    }

    // Try GZIP
    let mut decoder = GzDecoder::new(data);
    let mut buf = Vec::new();
    if decoder.read_to_end(&mut buf).is_ok() {
        if let Ok(pkgs) = parse_tar(&buf) {
            return Ok(pkgs);
        }
    }

    // Try raw TAR
    parse_tar(data)
}

fn parse_tar(data: &[u8]) -> Result<PackageBuildDates, String> {
    let mut pkgs = HashMap::new();
    let cursor = std::io::Cursor::new(data);
    let mut archive = Archive::new(cursor);
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?;
        let name_str = path.to_str().map(|s| s.to_string());
        if let Some(name) = name_str {
            if !name.ends_with("/desc") && name != "desc" {
                continue;
            }
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).map_err(|e| e.to_string())?;
            if let Some(ts) = extract_build_date(&contents) {
                // package name is folder before /desc
                let pkg_name = name
                    .trim_end_matches("/desc")
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                pkgs.insert(pkg_name, ts);
            }
        }
    }
    Ok(PackageBuildDates { packages: pkgs })
}

fn extract_build_date(desc: &[u8]) -> Option<i64> {
    let text = String::from_utf8_lossy(desc);
    let lines: Vec<&str> = text.lines().collect();
    for i in 0..lines.len().saturating_sub(1) {
        if lines[i].trim() == "%BUILDDATE%" {
            let val = lines[i + 1].trim();
            if let Ok(ts) = val.parse::<i64>() {
                return Some(ts);
            }
        }
    }
    None
}

pub fn calculate_freshness_score(
    mirror: &PackageBuildDates,
    reference: &PackageBuildDates,
) -> (f64, usize) {
    let mut score = 0.0;
    let mut compared = 0;
    for (pkg, ref_ts) in reference.packages.iter() {
        if let Some(m_ts) = mirror.packages.get(pkg) {
            compared += 1;
            if m_ts > ref_ts {
                score += 2.0;
            } else if m_ts == ref_ts {
                score += 1.0;
            }
        }
    }
    if compared == 0 {
        (0.0, 0)
    } else {
        (score / compared as f64, compared)
    }
}
