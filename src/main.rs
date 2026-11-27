mod config;
mod countries;
mod mirror;
mod speed_test;
mod target_configs;
mod targets;

use crate::config::{AppError, Config, FetchMirrors};
use crate::speed_test::{SpeedTestResult, SpeedTestResults, test_speed_by_countries};
use chrono::prelude::*;
use config::LogFormatter;
use itertools::Itertools;
use mirror::Mirror;
use nix::unistd::Uid;
use std::env;
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::prelude::*;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt, fchown};
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

fn preserve_ownership(file: &fs::File, uid: u32, gid: u32) -> io::Result<()> {
    let tmp_meta = file.metadata()?;
    let want_uid = (tmp_meta.uid() != uid).then_some(uid);
    let want_gid = (tmp_meta.gid() != gid).then_some(gid);
    if want_uid.is_none() && want_gid.is_none() {
        return Ok(());
    }
    match fchown(file, want_uid, want_gid) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            if want_uid.is_some() && want_gid.is_some() {
                let _ = fchown(file, None, want_gid);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn write_file_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    let existing = match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("refusing to save to symlink: {}", path.display()),
            ));
        }
        Ok(meta) if meta.file_type().is_file() => {
            OpenOptions::new().write(true).open(path)?;
            Some((meta.permissions().mode() & 0o777, meta.uid(), meta.gid()))
        }
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("refusing to save to non-regular file: {}", path.display()),
            ));
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };

    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };

    let (mut file, tmp_path) = {
        let mut created = None;
        for _ in 0..64 {
            let candidate = parent.join(format!(
                ".rate-mirrors-{}-{:016x}.tmp",
                std::process::id(),
                rand::random::<u64>()
            ));
            let mut opts = OpenOptions::new();
            opts.write(true).create_new(true);
            if existing.is_some() {
                opts.mode(0o600);
            }
            match opts.open(&candidate) {
                Ok(file) => {
                    created = Some((file, candidate));
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        created.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "failed to create unique temporary file",
            )
        })?
    };

    let result = (|| -> io::Result<()> {
        file.write_all(contents)?;
        if let Some((mode, uid, gid)) = existing {
            preserve_ownership(&file, uid, gid)?;
            file.set_permissions(fs::Permissions::from_mode(mode))?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

struct OutputSink<'a, T: LogFormatter> {
    filename: Option<String>,
    output_lines: Option<Vec<String>>,
    formatter: &'a T,
    comments_enabled: bool,
    comments_in_file_enabled: bool,
    mirror_count: usize,
}

impl<'a, T: LogFormatter> OutputSink<'a, T> {
    pub fn new(
        formatter: &'a T,
        filename: Option<&str>,
        comments_enabled: bool,
        comments_in_file_enabled: bool,
    ) -> Result<Self, io::Error> {
        let output = match filename {
            Some(filename) => Self {
                formatter,
                filename: Some(filename.to_string()),
                output_lines: Some(Vec::new()),
                comments_enabled,
                comments_in_file_enabled,
                mirror_count: 0,
            },
            None => Self {
                formatter,
                filename: None,
                output_lines: None,
                comments_enabled,
                comments_in_file_enabled,
                mirror_count: 0,
            },
        };
        Ok(output)
    }

    fn write_stdout_line(&mut self, line: &str) -> Result<(), AppError> {
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{}", line).map_err(|err| {
            if err.kind() == io::ErrorKind::BrokenPipe {
                AppError::StdoutBrokenPipe
            } else {
                AppError::IoError(err)
            }
        })
    }

    pub fn display_comment(&mut self, line: impl Display) -> Result<(), AppError> {
        if self.comments_enabled {
            let s = self.formatter.format_comment(line);
            self.write_stdout_line(&s)?;
            if self.comments_in_file_enabled {
                if let Some(output_lines) = &mut self.output_lines {
                    output_lines.push(s);
                }
            }
        }
        Ok(())
    }

    pub fn display_mirror(&mut self, mirror: &Mirror) -> Result<(), AppError> {
        let s = self.formatter.format_mirror(&mirror);
        self.write_stdout_line(&s)?;
        if let Some(output_lines) = &mut self.output_lines {
            output_lines.push(s);
        }
        self.mirror_count += 1;
        Ok(())
    }

    pub fn save_to_file(&mut self) -> Result<(), io::Error> {
        if let Some(output_lines) = &mut self.output_lines {
            if let Some(filename) = self.filename.as_ref() {
                let mut contents = output_lines.join("\n");
                contents.push('\n');
                write_file_atomically(Path::new(filename), contents.as_bytes())?;
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), AppError> {
    match run() {
        Err(AppError::StdoutBrokenPipe) => Ok(()),
        result => result,
    }
}

fn run() -> Result<(), AppError> {
    let config = Arc::new(Config::new());
    if !config.allow_root && Uid::effective().is_root() {
        return Err(AppError::Root);
    }
    let max_mirrors_to_output = config.max_mirrors_to_output.clone();
    let disable_untested_fallback = config.disable_untested_fallback;

    let ref formatter = Arc::clone(&config).target;
    let mut output = OutputSink::new(
        formatter,
        config.save_to_file.as_deref(),
        !config.disable_comments,
        !config.disable_comments_in_file,
    )?;

    output.display_comment(format!("STARTED AT: {}", Local::now()))?;
    output.display_comment(format!("VERSION: {}", env!("CARGO_PKG_VERSION")))?;
    output.display_comment(format!("ARGS: {}", env::args().join(" ")))?;

    let (tx_progress, rx_progress) = mpsc::channel::<String>();
    let (tx_results, rx_results) = mpsc::channel::<SpeedTestResults>();
    let (tx_mirrors, rx_mirrors) = mpsc::channel::<Mirror>();

    let thread_handle = thread::spawn(move || -> Result<(), AppError> {
        let mut mirrors = config.target.fetch_mirrors(tx_progress.clone())?;

        // Centralized protocol filtering
        let before_protocol = mirrors.len();
        mirrors.retain(|m| config.is_protocol_allowed_for_url(&m.url));
        if mirrors.len() < before_protocol {
            tx_progress
                .send(format!(
                    "PROTOCOL FILTER: {} -> {} mirrors",
                    before_protocol,
                    mirrors.len()
                ))
                .unwrap();
        }

        // Country filtering before dedup so excluded-country duplicates
        // don't shadow valid mirrors from non-excluded countries
        let before_country = mirrors.len();
        mirrors.retain(|m| {
            !config.is_country_excluded(m.country.map(|c| c.code.as_str()).unwrap_or("zz"))
        });
        if mirrors.len() < before_country {
            tx_progress
                .send(format!(
                    "COUNTRY FILTER: {} -> {} mirrors",
                    before_country,
                    mirrors.len()
                ))
                .unwrap();
        }

        // Prefer https over http when both are available for the same host
        mirrors.sort_by_key(|m| match m.url.scheme() {
            "https" => 0,
            "http" => 1,
            _ => 2,
        });

        // Deduplicate mirrors by host+port+path (keeps first = preferred protocol)
        let before_dedup = mirrors.len();
        let mut seen = std::collections::HashSet::new();
        mirrors.retain(|m| {
            let key = format!(
                "{}{}{}",
                m.url.host_str().unwrap_or(""),
                m.url.port().map(|p| format!(":{}", p)).unwrap_or_default(),
                m.url.path()
            );
            seen.insert(key)
        });
        if mirrors.len() < before_dedup {
            tx_progress
                .send(format!(
                    "DEDUP: {} -> {} mirrors",
                    before_dedup,
                    mirrors.len()
                ))
                .unwrap();
        }

        // sending filtered mirrors back so we have a fallback in case if all tests fail
        for mirror in mirrors.iter().cloned() {
            tx_mirrors.send(mirror).unwrap();
        }

        tx_progress
            .send(format!("MIRRORS LEFT AFTER FILTERING: {}", mirrors.len()))
            .unwrap();

        test_speed_by_countries(mirrors, config, tx_progress, tx_results);
        Ok(())
    });

    for progress in rx_progress.into_iter() {
        if let Err(err) = output.display_comment(progress) {
            // Exit before dropping rx_progress/rx_results/rx_mirrors so live worker
            // threads cannot hit send().unwrap() and SIGABRT under panic = "abort".
            match err {
                AppError::StdoutBrokenPipe => std::process::exit(0),
                err => {
                    eprintln!("Error: {err:?}");
                    std::process::exit(1);
                }
            }
        }
    }

    thread_handle.join().unwrap()?;

    let results: Vec<_> = rx_results.iter().flatten().collect();

    if results.is_empty() {
        let untested_mirrors: Vec<Mirror> = rx_mirrors.into_iter().collect();
        if untested_mirrors.len() == 0 {
            output.display_comment("==== NO MIRRORS AFTER FILTERING ====")?;
            return Err(AppError::NoMirrorsAfterFiltering);
        }
        if disable_untested_fallback {
            output.display_comment("==== ALL SPEED TESTS FAILED ====")?;
            return Err(AppError::SpeedTestsFailed);
        }
        output.display_comment("==== FAILED TO TEST SPEEDS, RETURNING UNTESTED MIRRORS ====")?;
        for mirror in untested_mirrors.into_iter() {
            output.display_mirror(&mirror)?;
        }
    } else {
        output.display_comment("==== RESULTS (top re-tested) ====")?;

        for (index, result) in results.iter().enumerate() {
            output.display_comment(format!("{:>3}. {}", index + 1, result))?;
        }

        output.display_comment(format!("FINISHED AT: {}", Local::now()))?;

        let it: Box<dyn Iterator<Item = SpeedTestResult>> = match max_mirrors_to_output {
            Some(n) => Box::new(results.into_iter().take(n)),
            None => Box::new(results.into_iter()),
        };

        for result in it {
            output.display_mirror(&result.item)?;
        }
    }

    if output.mirror_count == 0 {
        return Err(AppError::BlankOutput);
    }
    output.save_to_file()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let path = env::temp_dir().join(format!(
                "rate-mirrors-test-{}-{}-{:016x}",
                label,
                std::process::id(),
                rand::random::<u64>()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn temp_files(&self) -> Vec<String> {
            fs::read_dir(&self.0)
                .unwrap()
                .filter_map(|e| {
                    let name = e.ok()?.file_name().to_string_lossy().into_owned();
                    name.starts_with(".rate-mirrors-").then_some(name)
                })
                .collect()
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn saves_new_file_with_umask_permissions() {
        let dir = TestDir::new("new");
        let reference = dir.0.join("ref");
        File::create(&reference).unwrap();
        let expected_mode = fs::metadata(&reference).unwrap().permissions().mode() & 0o777;

        let dest = dir.0.join("mirrorlist");
        write_file_atomically(&dest, b"Server = https://example/\n").unwrap();

        assert_eq!(fs::read_to_string(&dest).unwrap(), "Server = https://example/\n");
        assert_eq!(
            fs::metadata(&dest).unwrap().permissions().mode() & 0o777,
            expected_mode
        );
        assert!(dir.temp_files().is_empty());
    }

    #[test]
    fn atomically_replaces_existing_file_and_preserves_mode() {
        let dir = TestDir::new("replace");
        for mode in [0o600_u32, 0o644_u32] {
            let dest = dir.0.join(format!("mirrorlist-{:o}", mode));
            fs::write(&dest, b"old\n").unwrap();
            fs::set_permissions(&dest, fs::Permissions::from_mode(mode)).unwrap();

            let mut old_fd = File::open(&dest).unwrap();
            write_file_atomically(&dest, b"new\n").unwrap();

            let mut old_bytes = Vec::new();
            old_fd.read_to_end(&mut old_bytes).unwrap();
            assert_eq!(old_bytes, b"old\n");
            assert_eq!(fs::read(&dest).unwrap(), b"new\n");
            assert_eq!(
                fs::symlink_metadata(&dest).unwrap().permissions().mode() & 0o777,
                mode
            );
        }
        assert!(dir.temp_files().is_empty());
    }

    #[test]
    fn refuses_symlink_and_dangling_symlink() {
        let dir = TestDir::new("symlink");
        let victim = dir.0.join("victim");
        fs::write(&victim, b"keep\n").unwrap();

        let link = dir.0.join("link");
        symlink(&victim, &link).unwrap();
        assert_eq!(
            write_file_atomically(&link, b"clobber\n").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(fs::read(&victim).unwrap(), b"keep\n");

        let missing = dir.0.join("missing");
        let dangling = dir.0.join("dangling");
        symlink(&missing, &dangling).unwrap();
        assert_eq!(
            write_file_atomically(&dangling, b"clobber\n").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(!missing.exists());
        assert!(dir.temp_files().is_empty());
    }

    #[test]
    fn cleans_up_temp_file_on_rename_failure() {
        let dir = TestDir::new("cleanup");
        // Trailing slash fails rename after staging a sibling temp file in `dir`.
        let bad_dest = PathBuf::from(format!("{}/missing/", dir.0.display()));
        assert!(write_file_atomically(&bad_dest, b"data\n").is_err());
        assert!(dir.temp_files().is_empty());
    }

    fn supplementary_gids() -> Vec<u32> {
        let Ok(output) = std::process::Command::new("id").arg("-G").output() else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    }

    #[test]
    fn refuses_read_only_file() {
        let dir = TestDir::new("readonly");
        let dest = dir.0.join("mirrorlist");
        fs::write(&dest, b"keep\n").unwrap();

        let (expected_uid, expected_gid) = if Uid::effective().is_root()
            && std::os::unix::fs::chown(&dest, Some(65534), Some(65534)).is_ok()
        {
            (65534, 65534)
        } else {
            let meta = fs::symlink_metadata(&dest).unwrap();
            (meta.uid(), meta.gid())
        };
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o444)).unwrap();

        if Uid::effective().is_root() {
            write_file_atomically(&dest, b"updated\n").unwrap();
            let meta = fs::symlink_metadata(&dest).unwrap();
            assert_eq!(fs::read(&dest).unwrap(), b"updated\n");
            assert_eq!(meta.permissions().mode() & 0o777, 0o444);
            assert_eq!((meta.uid(), meta.gid()), (expected_uid, expected_gid));
        } else {
            assert_eq!(
                write_file_atomically(&dest, b"clobber\n")
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::PermissionDenied
            );
            assert_eq!(fs::read(&dest).unwrap(), b"keep\n");
            assert_eq!(
                fs::symlink_metadata(&dest).unwrap().permissions().mode() & 0o777,
                0o444
            );
        }
        assert!(dir.temp_files().is_empty());
    }

    #[test]
    fn preserves_ownership_on_replace() {
        let dir = TestDir::new("ownership");
        let dest = dir.0.join("mirrorlist");
        fs::write(&dest, b"old\n").unwrap();

        let initial = fs::symlink_metadata(&dest).unwrap();
        let initial_uid = initial.uid();
        let initial_gid = initial.gid();

        let alt_gid = supplementary_gids()
            .into_iter()
            .find(|&gid| gid != initial_gid && std::os::unix::fs::chown(&dest, None, Some(gid)).is_ok());
        let expected_gid = alt_gid.unwrap_or(initial_gid);

        write_file_atomically(&dest, b"new\n").unwrap();
        let replaced = fs::symlink_metadata(&dest).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"new\n");
        assert_eq!((replaced.uid(), replaced.gid()), (initial_uid, expected_gid));
        assert!(dir.temp_files().is_empty());

        if !Uid::effective().is_root() {
            if let Some(alt_gid) = alt_gid {
                let fallback = File::create(dir.0.join("fallback")).unwrap();
                preserve_ownership(&fallback, initial_uid.wrapping_add(1), alt_gid).unwrap();
                let meta = fallback.metadata().unwrap();
                assert_eq!((meta.uid(), meta.gid()), (initial_uid, alt_gid));
            }
        }
    }
}
