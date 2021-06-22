use crate::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalChange {
    pub id: String,
    pub relative_path: String,
    pub change_type: String, //edit, add, delete
}

impl LocalChange {
    pub fn new(canonical_relative_path: String, change_type: String) -> Self {
        //todo: hash is case-sensitive - could be a problem
        let id = hash_string(&canonical_relative_path);
        Self {
            id,
            relative_path: canonical_relative_path,
            change_type,
        }
    }
}

pub fn save_local_change(workspace_root: &Path, change_spec: &LocalChange) -> Result<(), String> {
    let local_edit_obj_path =
        workspace_root.join(format!(".lsc/local_edits/{}.json", &change_spec.id));

    match serde_json::to_string(&change_spec) {
        Ok(json_spec) => {
            write_file(&local_edit_obj_path, json_spec.as_bytes())?;
        }
        Err(e) => {
            return Err(format!("Error formatting local change spec: {}", e));
        }
    }
    Ok(())
}

pub fn find_local_change(
    workspace_root: &Path,
    canonical_relative_path: &str,
) -> SearchResult<LocalChange, String> {
    let id = hash_string(canonical_relative_path);
    let path = workspace_root.join(format!(".lsc/local_edits/{}.json", id));
    if !path.exists() {
        return SearchResult::None;
    }

    match read_text_file(&path) {
        Ok(contents) => {
            let parsed: serde_json::Result<LocalChange> = serde_json::from_str(&contents);
            match parsed {
                Ok(edit) => SearchResult::Ok(edit),
                Err(e) => SearchResult::Err(format!("Error parsing {:?}: {}", path.display(), e)),
            }
        }
        Err(e) => SearchResult::Err(format!("Error reading {}: {}", path.display(), e)),
    }
}

pub fn read_local_changes(workspace_root: &Path) -> Result<Vec<LocalChange>, String> {
    let local_edits_dir = workspace_root.join(".lsc/local_edits");
    let mut res = Vec::new();
    match local_edits_dir.read_dir() {
        Ok(dir_iterator) => {
            for entry_res in dir_iterator {
                match entry_res {
                    Ok(entry) => {
                        let parsed: serde_json::Result<LocalChange> =
                            serde_json::from_str(&read_text_file(&entry.path())?);
                        match parsed {
                            Ok(edit) => {
                                res.push(edit);
                            }
                            Err(e) => {
                                return Err(format!("Error parsing {:?}: {}", entry.path(), e))
                            }
                        }
                    }
                    Err(e) => return Err(format!("Error reading local edit entry: {}", e)),
                }
            }
        }
        Err(e) => {
            return Err(format!(
                "Error reading directory {:?}: {}",
                local_edits_dir, e
            ))
        }
    }
    Ok(res)
}

pub fn find_local_changes_command() -> Result<Vec<LocalChange>, String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    read_local_changes(&workspace_root)
}

pub fn clear_local_change(workspace_root: &Path, change: &LocalChange) -> Result<(), String> {
    let change_path = workspace_root.join(format!(".lsc/local_edits/{}.json", &change.id));
    if let Err(e) = fs::remove_file(&change_path) {
        return Err(format!(
            "Error clearing local change {}: {}",
            change_path.display(),
            e
        ));
    }
    Ok(())
}

pub fn clear_local_changes(workspace_root: &Path, local_changes: &[LocalChange]) {
    for change in local_changes {
        let change_path = workspace_root.join(format!(".lsc/local_edits/{}.json", &change.id));
        if let Err(e) = fs::remove_file(change_path) {
            println!(
                "Error clearing local change {}: {}",
                change.relative_path, e
            );
        }
    }
}

pub fn track_new_file(path_specified: &Path) -> Result<(), String> {
    let abs_path = make_path_absolute(path_specified);
    if let Err(e) = fs::metadata(&abs_path) {
        return Err(format!(
            "Error reading file metadata {}: {}",
            &abs_path.display(),
            e
        ));
    }
    let workspace_root = find_workspace_root(&abs_path)?;
    //todo: make sure the file does not exist in the current tree hierarchy

    let relative_path = make_canonical_relative_path(&workspace_root, &abs_path)?;

    assert_not_locked(&workspace_root, &abs_path)?;
    let local_change = LocalChange::new(relative_path, String::from("add"));

    save_local_change(&workspace_root, &local_change)
}

pub fn edit_file_command(path_specified: &Path) -> Result<(), String> {
    let abs_path = make_path_absolute(path_specified);
    if let Err(e) = fs::metadata(&abs_path) {
        return Err(format!(
            "Error reading file metadata {}: {}",
            &abs_path.display(),
            e
        ));
    }

    let workspace_root = find_workspace_root(&abs_path)?;
    //todo: make sure file is tracked by finding it in the current tree hierarchy
    assert_not_locked(&workspace_root, &abs_path)?;

    let relative_path = make_canonical_relative_path(&workspace_root, &abs_path)?;
    let local_change = LocalChange::new(relative_path, String::from("edit"));
    save_local_change(&workspace_root, &local_change)?;
    make_file_read_only(&abs_path, false)
}
