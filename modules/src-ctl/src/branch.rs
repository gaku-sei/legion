use crate::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct Branch {
    pub name: String,
    pub head: String, //commit id
    pub parent: String,
    pub lock_domain_id: String,
}

impl Branch {
    pub fn new(name: String, head: String, parent: String, lock_domain_id: String) -> Self {
        Self {
            name,
            head,
            parent,
            lock_domain_id,
        }
    }
}

fn write_branch_spec(file_path: &Path, branch: &Branch) -> Result<(), String> {
    match serde_json::to_string(branch) {
        Ok(json) => write_file(file_path, json.as_bytes()),
        Err(e) => Err(format!("Error formatting branch {:?}: {}", branch, e)),
    }
}

pub fn save_new_branch_to_repo(repo: &Path, branch: &Branch) -> Result<(), String> {
    let file_path = repo.join("branches").join(branch.name.clone() + ".json");
    match serde_json::to_string(branch) {
        Ok(json) => write_new_file(&file_path, json.as_bytes()),
        Err(e) => Err(format!("Error formatting branch {:?}: {}", branch, e)),
    }
}

pub fn save_branch_to_repo(repo: &Path, branch: &Branch) -> Result<(), String> {
    let file_path = repo.join("branches").join(branch.name.clone() + ".json");
    write_branch_spec(&file_path, branch)
}

pub fn save_current_branch(workspace_root: &Path, branch: &Branch) -> Result<(), String> {
    let file_path = workspace_root.join(".lsc/branch.json");
    write_branch_spec(&file_path, branch)
}

pub fn read_current_branch(workspace_root: &Path) -> Result<Branch, String> {
    let file_path = workspace_root.join(".lsc/branch.json");
    read_branch(&file_path)
}

pub fn read_branch_from_repo(repo: &Path, name: &str) -> Result<Branch, String> {
    let file_path = repo.join(format!("branches/{}.json", name));
    read_branch(&file_path)
}

pub fn find_branch(repo: &Path, name: &str) -> SearchResult<Branch, String> {
    let file_path = repo.join(format!("branches/{}.json", name));
    if !file_path.exists() {
        return SearchResult::None;
    }
    match read_branch(&file_path) {
        Ok(branch) => SearchResult::Ok(branch),
        Err(e) => SearchResult::Err(e),
    }
}

pub fn read_branch(branch_file_path: &Path) -> Result<Branch, String> {
    let parsed: serde_json::Result<Branch> =
        serde_json::from_str(&read_text_file(branch_file_path)?);
    match parsed {
        Ok(branch) => Ok(branch),
        Err(e) => Err(format!(
            "Error reading branch spec {}: {}",
            branch_file_path.display(),
            e
        )),
    }
}

pub fn create_branch_command(name: &str) -> Result<(), String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let old_branch = read_current_branch(&workspace_root)?;
    let new_branch = Branch::new(
        String::from(name),
        old_branch.head.clone(),
        old_branch.name,
        old_branch.lock_domain_id,
    );
    save_new_branch_to_repo(&workspace_spec.repository, &new_branch)?;
    save_current_branch(&workspace_root, &new_branch)
}

pub fn read_branches(repo: &Path) -> Result<Vec<Branch>, String> {
    let mut res = Vec::new();
    let branches_dir = repo.join("branches");
    match branches_dir.read_dir() {
        Ok(dir_iterator) => {
            for entry_res in dir_iterator {
                match entry_res {
                    Ok(entry) => {
                        let parsed: serde_json::Result<Branch> =
                            serde_json::from_str(&read_text_file(&entry.path())?);
                        match parsed {
                            Ok(branch) => {
                                res.push(branch);
                            }
                            Err(e) => {
                                return Err(format!(
                                    "Error parsing {}: {}",
                                    entry.path().display(),
                                    e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Error reading branch entry: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            return Err(format!(
                "Error reading {} directory: {}",
                branches_dir.display(),
                e
            ));
        }
    }
    Ok(res)
}

pub fn list_branches_command() -> Result<(), String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let repo = &workspace_spec.repository;
    for branch in read_branches(repo)? {
        println!(
            "{} head:{} parent:{}",
            branch.name, branch.head, branch.parent
        );
    }
    Ok(())
}
