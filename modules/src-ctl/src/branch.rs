use crate::{sql::execute_sql, *};
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

    pub fn from_json(contents: &str) -> Result<Self, String> {
        let parsed: serde_json::Result<Self> = serde_json::from_str(contents);
        match parsed {
            Ok(branch) => Ok(branch),
            Err(e) => Err(format!("Error parsing branch spec {}", e)),
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        match serde_json::to_string(&self) {
            Ok(json) => Ok(json),
            Err(e) => Err(format!("Error formatting branch {:?}: {}", self.name, e)),
        }
    }
}

pub fn init_branch_database(sql_connection: &mut sqlx::AnyConnection) -> Result<(), String> {
    let sql = "CREATE TABLE branches(name VARCHAR(255), head VARCHAR(255), parent VARCHAR(255), lock_domain_id VARCHAR(64));
         CREATE UNIQUE INDEX branch_name on branches(name);
        ";
    if let Err(e) = execute_sql(sql_connection, sql) {
        return Err(format!("Error creating branch table and index: {}", e));
    }
    Ok(())
}

fn write_branch_spec(file_path: &Path, branch: &Branch) -> Result<(), String> {
    write_file(file_path, branch.to_json()?.as_bytes())
}

pub fn save_current_branch(workspace_root: &Path, branch: &Branch) -> Result<(), String> {
    let file_path = workspace_root.join(".lsc/branch.json");
    write_branch_spec(&file_path, branch)
}

pub fn read_current_branch(workspace_root: &Path) -> Result<Branch, String> {
    let file_path = workspace_root.join(".lsc/branch.json");
    read_branch(&file_path)
}

pub fn read_branch(branch_file_path: &Path) -> Result<Branch, String> {
    Branch::from_json(&read_text_file(branch_file_path)?)
}

pub async fn create_branch_command(name: &str) -> Result<(), String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let connection = connect_to_server(&workspace_spec).await?;
    let query = connection.query();
    let old_branch = read_current_branch(&workspace_root)?;
    let new_branch = Branch::new(
        String::from(name),
        old_branch.head.clone(),
        old_branch.name,
        old_branch.lock_domain_id,
    );
    query.insert_branch(&new_branch).await?;
    save_current_branch(&workspace_root, &new_branch)
}

pub async fn list_branches_command() -> Result<(), String> {
    let current_dir = std::env::current_dir().unwrap();
    let workspace_root = find_workspace_root(&current_dir)?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let connection = connect_to_server(&workspace_spec).await?;
    let query = connection.query();
    for branch in query.read_branches().await? {
        println!(
            "{} head:{} parent:{}",
            branch.name, branch.head, branch.parent
        );
    }
    Ok(())
}
