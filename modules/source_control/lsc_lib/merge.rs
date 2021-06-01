use crate::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct MergePending {
    pub id: String,
    pub relative_path: PathBuf,
    pub base_commit_id: String,
    pub theirs_commit_id: String,
}

impl MergePending {
    pub fn new(
        relative_path: PathBuf,
        base_commit_id: String,
        theirs_commit_id: String,
    ) -> MergePending {
        let id = uuid::Uuid::new_v4().to_string();
        MergePending {
            id,
            relative_path,
            base_commit_id,
            theirs_commit_id,
        }
    }
}

pub fn save_merge_pending(
    workspace_root: &Path,
    merge_pending: &MergePending,
) -> Result<(), String> {
    let file_path = workspace_root.join(format!(".lsc/merge_pending/{}.json", &merge_pending.id));
    match serde_json::to_string(&merge_pending) {
        Ok(json) => {
            write_file(&file_path, json.as_bytes())?;
        }
        Err(e) => {
            return Err(format!("Error formatting merge pending: {}", e));
        }
    }
    Ok(())
}

pub fn clear_merge_pending(
    workspace_root: &Path,
    merge_pending: &MergePending,
) -> Result<(), String> {
    let file_path = workspace_root.join(format!(".lsc/merge_pending/{}.json", &merge_pending.id));
    if let Err(e) = fs::remove_file(&file_path) {
        return Err(format!(
            "Error clearing merge pending {}: {}",
            file_path.display(),
            e
        ));
    }
    Ok(())
}

fn find_merge_pending(workspace_root: &Path, relative_path: &Path) -> Result<MergePending, String> {
    let merges_pending_dir = workspace_root.join(".lsc/merge_pending");
    match merges_pending_dir.read_dir() {
        Ok(dir_iterator) => {
            for entry_res in dir_iterator {
                match entry_res {
                    Ok(entry) => {
                        let parsed: serde_json::Result<MergePending> =
                            serde_json::from_str(&read_text_file(&entry.path())?);
                        match parsed {
                            Ok(merge) => {
                                if merge.relative_path == relative_path {
                                    return Ok(merge);
                                }
                            }
                            Err(e) => {
                                return Err(format!("Error parsing {:?}: {}", entry.path(), e));
                            }
                        }
                    }
                    Err(e) => return Err(format!("Error reading pending merge entry: {}", e)),
                }
            }
        }
        Err(e) => {
            return Err(format!(
                "Error reading directory {:?}: {}",
                merges_pending_dir, e
            ))
        }
    }
    Err(format!(
        "local change {} not found",
        relative_path.display()
    ))
}

fn read_merges_pending(workspace_root: &Path) -> Result<Vec<MergePending>, String> {
    let merges_pending_dir = workspace_root.join(".lsc/merge_pending");
    let mut res = Vec::new();
    match merges_pending_dir.read_dir() {
        Ok(dir_iterator) => {
            for entry_res in dir_iterator {
                match entry_res {
                    Ok(entry) => {
                        let parsed: serde_json::Result<MergePending> =
                            serde_json::from_str(&read_text_file(&entry.path())?);
                        match parsed {
                            Ok(merge) => {
                                res.push(merge);
                            }
                            Err(e) => {
                                return Err(format!("Error parsing {:?}: {}", entry.path(), e))
                            }
                        }
                    }
                    Err(e) => return Err(format!("Error reading merge pending entry: {}", e)),
                }
            }
        }
        Err(e) => {
            return Err(format!(
                "Error reading directory {:?}: {}",
                merges_pending_dir, e
            ))
        }
    }
    Ok(res)
}

pub fn find_merges_pending_command() -> Result<Vec<MergePending>, String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    read_merges_pending(&workspace_root)
}

//todo: move to repo
pub fn find_file_hash_at_commit(
    repo: &Path,
    relative_path: &Path,
    commit_id: &str,
) -> Result<String, String> {
    let commit = read_commit(repo, commit_id)?;
    let root_tree = read_tree(repo, &commit.root_hash)?;
    let parent_dir = relative_path.parent().expect("no parent to path provided");
    let dir_tree = fetch_tree_subdir(repo, &root_tree, &parent_dir)?;
    let file_node = dir_tree.find_file_node(
        relative_path
            .file_name()
            .expect("no file name in path specified")
            .to_str()
            .expect("invalid file name"),
    )?;
    Ok(file_node.hash.clone())
}

fn run_merge_program(
    relative_path: &Path,
    abs_path: &str,
    theirs_path: &str,
    base_path: &str,
    output_path: &str,
) -> Result<(), String> {
    let config = Config::read_config()?;
    match config.find_merge_command(&relative_path) {
        Some(mut external_command_vec) => {
            for item in &mut external_command_vec[..] {
                *item = item.replace("%local", &abs_path);
                *item = item.replace("%theirs", &theirs_path);
                *item = item.replace("%base", &base_path);
                *item = item.replace("%output", &output_path);
            }

            match Command::new(&external_command_vec[0])
                .args(&external_command_vec[1..])
                .output()
            {
                Ok(output) => {
                    let mut out = std::io::stdout();
                    out.write_all(&output.stdout).unwrap();
                    out.flush().unwrap();

                    let mut err = std::io::stderr();
                    err.write_all(&output.stderr).unwrap();
                    err.flush().unwrap();
                }
                Err(e) => {
                    return Err(format!(
                        "Error executing external command {:?}: {}",
                        external_command_vec, e
                    ));
                }
            }
        }
        None => {
            return Err(format!(
                "No merge command corresponding to {} was found in {}",
                relative_path.display(),
                Config::config_file_path().unwrap().display()
            ));
        }
    }
    Ok(())
}

fn run_diffy_merge(yours_path: &Path, theirs_path: &Path, base_path: &Path) -> Result<(), String> {
    let yours = read_bin_file(&yours_path)?;
    let theirs = read_bin_file(&theirs_path)?;
    let base = read_bin_file(&base_path)?;
    match diffy::merge_bytes(&base, &yours, &theirs) {
        Ok(merged_contents) => {
            write_file(&yours_path, &merged_contents)?;
            println!("Merge completed, {} updated", yours_path.display());
        }
        Err(conflicts) => {
            write_file(&yours_path, &conflicts)?;
            println!(
                "Merge *not* completed, please resolve conflicts in {}",
                yours_path.display()
            );
        }
    }
    Ok(())
}

pub fn merge_file_command(p: &Path, allow_tools: bool) -> Result<(), String> {
    let abs_path = make_path_absolute(p);
    let workspace_root = find_workspace_root(&abs_path)?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let repo = &workspace_spec.repository;
    let relative_path = path_relative_to(&abs_path, workspace_root)?;
    let merge_pending = find_merge_pending(&workspace_root, &relative_path)?;
    let base_file_hash = find_file_hash_at_commit(
        &workspace_spec.repository,
        &relative_path,
        &merge_pending.base_commit_id,
    )?;
    let base_temp_file = download_temp_file(&repo, &workspace_root, &base_file_hash)?;
    let theirs_file_hash = find_file_hash_at_commit(
        &workspace_spec.repository,
        &relative_path,
        &merge_pending.theirs_commit_id,
    )?;
    let theirs_temp_file = download_temp_file(&repo, &workspace_root, &theirs_file_hash)?;
    let tmp_dir = workspace_root.join(".lsc/tmp");
    let output_temp_file = TempPath {
        path: tmp_dir.join(format!("merge_output_{}", uuid::Uuid::new_v4().to_string())),
    };
    if !allow_tools {
        run_diffy_merge(&abs_path, &theirs_temp_file.path, &base_temp_file.path)?;
        clear_merge_pending(&workspace_root, &merge_pending)?;
        return Ok(());
    }

    run_merge_program(
        &relative_path,
        abs_path.to_str().unwrap(),
        theirs_temp_file.path.to_str().unwrap(),
        base_temp_file.path.to_str().unwrap(),
        output_temp_file.path.to_str().unwrap(),
    )?;
    if let Err(e) = fs::copy(&output_temp_file.path, &abs_path) {
        return Err(format!(
            "Error copying {} to {}: {}",
            output_temp_file.path.display(),
            abs_path.display(),
            e
        ));
    }
    println!("Merge accepted, {} updated", abs_path.display());
    clear_merge_pending(&workspace_root, &merge_pending)?;
    Ok(())
}
