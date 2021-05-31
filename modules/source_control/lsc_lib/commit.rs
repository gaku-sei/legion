use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug)]
pub struct HashedChange {
    pub relative_path: PathBuf,
    pub hash: String,
    pub change_type: String, //edit, add, delete
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub changes: Vec<HashedChange>,
    pub root_hash: String,
    pub parents: Vec<String>,
}

impl Commit {
    pub fn new(
        message: String,
        changes: Vec<HashedChange>,
        root_hash: String,
        parents: Vec<String>,
    ) -> Commit {
        let id = uuid::Uuid::new_v4().to_string();
        Commit {
            id,
            message,
            changes,
            root_hash,
            parents,
        }
    }
}

pub fn save_commit(repo: &Path, commit: &Commit) -> Result<(), String> {
    let file_path = repo.join("commits").join(commit.id.to_owned() + ".json");
    match serde_json::to_string(&commit) {
        Ok(json) => {
            write_file(&file_path, json.as_bytes())?;
        }
        Err(e) => {
            return Err(format!("Error formatting commit {:?}: {}", commit, e));
        }
    }
    Ok(())
}

pub fn read_commit(repo: &Path, id: &str) -> Result<Commit, String> {
    let file_path = repo.join(format!("commits/{}.json", id));
    let parsed: serde_json::Result<Commit> = serde_json::from_str(&read_text_file(&file_path)?);
    match parsed {
        Ok(commit) => Ok(commit),
        Err(e) => Err(format!("Error reading commit {}: {}", id, e)),
    }
}

fn write_blob(file_path: &Path, contents: &[u8]) -> Result<(), String> {
    if fs::metadata(file_path).is_ok() {
        //blob already exists
        return Ok(());
    }

    lz4_compress_to_file(file_path, contents)
}

fn upload_localy_edited_blobs(
    workspace_root: &Path,
    workspace_spec: &Workspace,
    local_changes: &[LocalChange],
) -> Result<Vec<HashedChange>, String> {
    let blob_dir = Path::new(&workspace_spec.repository).join("blobs");
    let mut res = Vec::<HashedChange>::new();
    for local_change in local_changes {
        let workspace_path = workspace_root.join(&local_change.relative_path);
        let local_file_contents = read_bin_file(&workspace_path)?;
        let hash = format!("{:X}", Sha256::digest(&local_file_contents));
        write_blob(&blob_dir.join(&hash), &local_file_contents)?;
        res.push(HashedChange {
            relative_path: local_change.relative_path.clone(),
            hash: hash.clone(),
            change_type: local_change.change_type.clone(),
        });
    }
    Ok(res)
}

fn make_local_files_read_only(
    workspace_root: &Path,
    changes: &[HashedChange],
) -> Result<(), String> {
    for change in changes {
        if change.change_type != "delete" {
            let full_path = workspace_root.join(&change.relative_path);
            match fs::metadata(&full_path) {
                Ok(meta) => {
                    let mut permissions = meta.permissions();
                    permissions.set_readonly(true);
                    if let Err(e) = fs::set_permissions(&full_path, permissions) {
                        return Err(format!(
                            "Error making file read only for {}: {}",
                            full_path.display(),
                            e
                        ));
                    }
                }
                Err(e) => {
                    return Err(format!(
                        "Error reading file metadata for {}: {}",
                        full_path.display(),
                        e
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn commit(message: &str) -> Result<(), String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let local_changes = find_local_changes(workspace_root)?;
    let hashed_changes =
        upload_localy_edited_blobs(workspace_root, &workspace_spec, &local_changes)?;

    let mut current_branch = read_current_branch(&workspace_root)?;
    let base_commit = read_commit(&workspace_spec.repository, &current_branch.head)?;
    let previous_root_tree = read_tree(&workspace_spec.repository, &base_commit.root_hash)?;

    let new_root_hash = update_tree_from_changes(
        previous_root_tree,
        &hashed_changes,
        &workspace_spec.repository,
    )?;

    let commit = Commit::new(
        String::from(message),
        hashed_changes,
        new_root_hash,
        Vec::from([base_commit.id]),
    );
    save_commit(&workspace_spec.repository, &commit)?;
    current_branch.head = commit.id;
    save_current_branch(&workspace_root, &current_branch)?;

    //todo: will need to lock to avoid races in updating branch in the database
    save_branch_to_repo(&workspace_spec.repository, &current_branch)?;

    if let Err(e) = make_local_files_read_only(&workspace_root, &commit.changes) {
        println!("Error making local files read only: {}", e);
    }
    clear_local_changes(&workspace_root, &local_changes);
    Ok(())
}
