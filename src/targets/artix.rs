use crate::config::{AppError, FetchMirrors, LogFormatter};
use crate::mirror::Mirror;
use crate::target_configs::artix::ArtixTarget;
use crate::targets::archlinux::fetch_archweb_mirrors;
use std::fmt::Display;
use std::sync::mpsc;

pub(crate) const ARTIX_TIER_1_MIRROR_SOURCE: &str =
    "https://status.artixlinux.org/mirrors/status/tier/1/json/";

impl LogFormatter for ArtixTarget {
    fn format_comment(&self, message: impl Display) -> String {
        format!("{}{}", self.comment_prefix, message)
    }

    fn format_mirror(&self, mirror: &Mirror) -> String {
        format!("Server = {}$repo/os/$arch", mirror.url)
    }
}

pub(crate) fn selected_mirror_source(target: &ArtixTarget) -> &str {
    if target.fetch_first_tier_only {
        ARTIX_TIER_1_MIRROR_SOURCE
    } else {
        &target.mirror_source
    }
}

impl FetchMirrors for ArtixTarget {
    fn fetch_mirrors(&self, tx_progress: mpsc::Sender<String>) -> Result<Vec<Mirror>, AppError> {
        fetch_archweb_mirrors(
            selected_mirror_source(self),
            self.fetch_mirrors_timeout,
            self.completion,
            self.max_delay,
            &self.sort_mirrors_by,
            &self.path_to_test,
            tx_progress,
        )
    }
}
