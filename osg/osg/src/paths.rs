use std::path::PathBuf;

use failure::{err_msg, Fallible};

const BASEDIR_DETECTION_ERROR: &str =
    "Failed to detect platform-dependent default directory for profile management";

pub fn default_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("prometheus"))
}

pub fn vault_path(parent_dir: Option<PathBuf>) -> Fallible<PathBuf> {
    let parent_dir = parent_dir.or_else(default_dir);
    parent_dir.map(|base| base.join("vault.dat")).ok_or_else(|| err_msg(BASEDIR_DETECTION_ERROR))
}

pub fn profile_repo_path(parent_dir: Option<PathBuf>) -> Fallible<PathBuf> {
    let parent_dir = parent_dir.or_else(default_dir);
    parent_dir.map(|base| base.join("profiles.dat")).ok_or_else(|| err_msg(BASEDIR_DETECTION_ERROR))
}

pub fn base_repo_path(parent_dir: Option<PathBuf>) -> Fallible<PathBuf> {
    let parent_dir = parent_dir.or_else(default_dir);
    parent_dir.map(|base| base.join("bases.dat")).ok_or_else(|| err_msg(BASEDIR_DETECTION_ERROR))
}
