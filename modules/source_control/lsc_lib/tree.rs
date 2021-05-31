use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::hash_map::HashMap;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeNode {
    pub name: PathBuf,
    pub hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tree {
    pub directory_nodes: Vec<TreeNode>,
    pub file_nodes: Vec<TreeNode>,
}

impl Tree {
    pub fn empty() -> Tree {
        Tree {
            directory_nodes: Vec::new(),
            file_nodes: Vec::new(),
        }
    }

    pub fn hash(&self) -> String {
        //std::hash::Hasher is not right here because it supports only 64 bit hashes
        let mut hasher = Sha256::new();
        for node in &self.directory_nodes {
            hasher.update(node.name.to_str().expect("invalid node name").as_bytes());
            hasher.update(&node.hash);
        }
        for node in &self.file_nodes {
            hasher.update(node.name.to_str().expect("invalid node name").as_bytes());
            hasher.update(&node.hash);
        }
        format!("{:X}", hasher.finalize())
    }

    pub fn add_or_update_file_node(&mut self, node: TreeNode) {
        self.file_nodes.push(node);
    }

    pub fn add_or_update_dir_node(&mut self, node: TreeNode) {
        self.directory_nodes.push(node);
    }

    pub fn remove_file_node(&mut self, node_name: &Path) {
        if let Some(index) = self.file_nodes.iter().position(|x| x.name == node_name) {
            self.file_nodes.swap_remove(index);
        }
    }
}

pub fn save_tree(repo: &Path, tree: &Tree, hash: &str) -> Result<(), String> {
    let file_path = repo.join("trees").join(String::from(hash) + ".json");
    match serde_json::to_string(&tree) {
        Ok(json) => {
            write_file(&file_path, json.as_bytes())?;
        }
        Err(e) => {
            return Err(format!("Error formatting tree {:?}: {}", tree, e));
        }
    }
    Ok(())
}

pub fn read_tree(repo: &Path, hash: &str) -> Result<Tree, String> {
    let file_path = repo.join(format!("trees/{}.json", hash));
    let parsed: serde_json::Result<Tree> = serde_json::from_str(&read_text_file(&file_path)?);
    match parsed {
        Ok(tree) => Ok(tree),
        Err(e) => Err(format!("Error reading tree {}: {}", hash, e)),
    }
}

// returns the hash of the updated root tree
pub fn update_tree_from_changes(
    _previous_version: Tree,
    local_changes: &[HashedChange],
    repo: &Path,
) -> Result<String, String> {
    //scan changes to get the list of trees to update
    let mut dir_to_update = BTreeSet::new();
    for change in local_changes {
        let parent = change
            .relative_path
            .parent()
            .expect("relative path with no parent");
        dir_to_update.insert(parent);
    }
    let root = Path::new("");
    //add ancestors
    for dir in dir_to_update.clone() {
        if let Some(mut parent) = dir.parent() {
            loop {
                dir_to_update.insert(parent);
                if parent == root {
                    break;
                }
                parent = parent.parent().expect("relative path with no parent");
            }
        }
    }
    let mut dir_to_update_by_length = Vec::<PathBuf>::new();
    for dir in dir_to_update {
        dir_to_update_by_length.push(dir.to_path_buf());
    }

    let mut parent_to_children_dir = HashMap::<PathBuf, Vec<TreeNode>>::new();
    //process leafs before parents to be able to patch parents with hash of children
    dir_to_update_by_length.sort_by_key(|a| core::cmp::Reverse(a.components().count()));
    for dir in dir_to_update_by_length {
        let mut tree = Tree::empty(); //todo: fetch previous version
        for change in local_changes {
            let parent = change
                .relative_path
                .parent()
                .expect("relative path with no parent");
            if dir == parent {
                //todo: handle edit & delete
                tree.add_or_update_file_node(TreeNode {
                    name: PathBuf::from(
                        change
                            .relative_path
                            .file_name()
                            .expect("error getting file name"),
                    ),
                    hash: change.hash.clone(),
                });
            }
        }
        //find dir's children, add them to the current tree
        if let Some(v) = parent_to_children_dir.get(&dir) {
            for node in v {
                tree.add_or_update_dir_node(node.clone());
            }
        }

        let dir_hash = tree.hash(); //important not to modify tree beyond this point

        //save the child for the parent to find
        if let Some(dir_parent) = dir.parent() {
            //save the child for the parent to find
            let key = dir_parent.to_path_buf();
            let name = dir
                .strip_prefix(dir_parent)
                .expect("Error getting directory name");
            let dir_node = TreeNode {
                name: name.to_path_buf(),
                hash: dir_hash.clone(),
            };
            match parent_to_children_dir.get_mut(&key) {
                Some(v) => {
                    v.push(dir_node);
                }
                None => {
                    parent_to_children_dir.insert(key, Vec::from([dir_node]));
                }
            }
        }

        save_tree(repo, &tree, &dir_hash)?;
        if dir.components().count() == 0 {
            return Ok(dir_hash);
        }
    }
    Err(String::from("root tree not processed"))
}

pub fn download_tree(repo: &Path, download_path: &Path, tree_hash: &str) -> Result<(), String> {
    let mut dir_to_process = Vec::from([TreeNode {
        name: download_path.to_path_buf(),
        hash: String::from(tree_hash),
    }]);
    let mut errors: Vec<String> = Vec::new();
    while !dir_to_process.is_empty() {
        let dir_node = dir_to_process.pop().expect("empty dir_to_process");
        let tree = read_tree(repo, &dir_node.hash)?;
        for relative_subdir_node in tree.directory_nodes {
            let abs_subdir_node = TreeNode {
                name: dir_node.name.join(relative_subdir_node.name),
                hash: relative_subdir_node.hash,
            };
            match std::fs::create_dir_all(&abs_subdir_node.name) {
                Ok(_) => {
                    dir_to_process.push(abs_subdir_node);
                }
                Err(e) => {
                    errors.push(format!(
                        "Error creating directory {}: {}",
                        abs_subdir_node.name.display(),
                        e
                    ));
                }
            }
        }
        for relative_file_node in tree.file_nodes {
            let abs_path = dir_node.name.join(relative_file_node.name);
            let blob_path = repo.join(format!("blobs/{}", relative_file_node.hash));
            if let Err(e) = lz4_decompress(&blob_path, &abs_path) {
                errors.push(format!(
                    "Error copying {} to {}: {}",
                    blob_path.display(),
                    abs_path.display(),
                    e
                ));
            }
        }
    }
    if !errors.is_empty() {
        let message = errors.join("\n");
        return Err(message);
    }
    Ok(())
}
