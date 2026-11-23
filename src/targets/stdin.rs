use crate::config::{AppError, FetchMirrors, LogFormatter};
use crate::mirror::{Mirror, MirrorInfo};
use crate::target_configs::stdin::StdinTarget;
use std::fmt::Display;
use std::io::{self, BufRead, Write};
use std::sync::mpsc;

impl LogFormatter for StdinTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        let display_url = mirror
            .url
            .join(&self.path_to_return)
            .unwrap_or_else(|_| mirror.url.clone());
        format!("{}{}", self.output_prefix, display_url)
    }
}

impl StdinTarget {
    fn parse_mirrors<R: BufRead>(&self, reader: R) -> Vec<Mirror> {
        reader
            .lines()
            .filter_map(|line| match line {
                Ok(line) => match MirrorInfo::parse(&line, &self.separator) {
                    Ok(info) => {
                        let Some(mirror) =
                            Mirror::with_test_path(info.url, &self.path_to_test, info.country)
                        else {
                            let _ = writeln!(io::stderr(), "Bad url: {}", line);
                            return None;
                        };
                        if mirror.url.join(&self.path_to_return).is_err() {
                            let _ = writeln!(io::stderr(), "Bad url: {}", mirror.url);
                            return None;
                        }
                        Some(mirror)
                    }
                    Err(err) => {
                        let _ = writeln!(io::stderr(), "{}", err);
                        None
                    }
                },
                Err(err) => {
                    let _ = writeln!(io::stderr(), "failed to read line: {}", err);
                    None
                }
            })
            .collect()
    }
}

impl FetchMirrors for StdinTarget {
    fn fetch_mirrors(&self, _tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        Ok(self.parse_mirrors(io::stdin().lock()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn skips_non_base_and_unjoinable_return_paths() {
        let target = StdinTarget {
            path_to_test: "extra.db".to_string(),
            path_to_return: "http:".to_string(),
            comment_prefix: "# ".to_string(),
            output_prefix: "Server = ".to_string(),
            separator: "\t".to_string(),
        };
        let input = "\
mailto:admin@example.com
data:text/plain,hello
https://https.example/repo/
http://http.example/repo/
";
        let mirrors = target.parse_mirrors(Cursor::new(input));
        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].url.as_str(), "http://http.example/repo/");
        assert_eq!(
            target.format_mirror(&mirrors[0]),
            "Server = http://http.example/repo/"
        );
    }
}
