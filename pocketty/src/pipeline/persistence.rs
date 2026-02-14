// to be called on main startup and quit; saves state of app so we can reload it later
use std::path::{Path, PathBuf};
use crate::pipeline::project::ProjectState;

const POCKETTY_DIR: &str = ".pocketty";
const PROJECT_FILE: &str = "project.json";

// <project_dir>/.pocketty/project.json
fn project_file_path(project_dir: &Path) -> PathBuf {
    project_dir.join(POCKETTY_DIR).join(PROJECT_FILE)
}

pub fn load_project(project_dir: &Path) -> Option<ProjectState> {
    let path = project_file_path(project_dir);
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

// Save the project state to disk, making the files if they don't exist already
pub fn save_project(project_dir: &Path, state: &ProjectState) -> anyhow::Result<()> {
    let path = project_file_path(project_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?; // create .pocketty/ if needed
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}
