use std::io::Write;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::{
    connect_to_server, download_temp_file, find_file_hash_at_commit, find_workspace_root,
    make_path_absolute, path_relative_to, read_current_branch, read_text_file, read_workspace_spec,
    Config, IndexBackend, LocalWorkspaceConnection, RepositoryConnection,
};

async fn reference_version_name_as_commit_id(
    repo_query: &dyn IndexBackend,
    workspace_connection: &mut LocalWorkspaceConnection,
    reference_version_name: &str,
) -> Result<String> {
    match reference_version_name {
        "base" => {
            let (_branch_name, commit_id) = read_current_branch(workspace_connection.sql()).await?;
            Ok(commit_id)
        }
        "latest" => {
            let (branch_name, _commit_id) = read_current_branch(workspace_connection.sql()).await?;
            let branch = repo_query.read_branch(&branch_name).await?;
            Ok(branch.head)
        }
        _ => Ok(String::from(reference_version_name)),
    }
}

async fn print_diff(
    connection: &RepositoryConnection,
    local_path: &Path,
    ref_file_hash: &str,
) -> Result<()> {
    let base_version_contents = connection.blob_storage().read_blob(ref_file_hash).await?;
    let base_version_contents =
        String::from_utf8(base_version_contents).context("error reading base version contents")?;

    let local_version_contents = read_text_file(local_path)?;
    let patch = diffy::create_patch(&base_version_contents, &local_version_contents);
    println!("{}", patch);
    Ok(())
}

pub async fn diff_file_command(
    path: &Path,
    reference_version_name: &str,
    allow_tools: bool,
) -> Result<()> {
    let abs_path = make_path_absolute(path);
    let workspace_root = find_workspace_root(&abs_path)?;
    let mut workspace_connection = LocalWorkspaceConnection::new(&workspace_root).await?;
    let workspace_spec = read_workspace_spec(&workspace_root)?;
    let connection = connect_to_server(&workspace_spec).await?;
    let relative_path = path_relative_to(&abs_path, &workspace_root)?;
    let ref_commit_id = reference_version_name_as_commit_id(
        connection.index_backend(),
        &mut workspace_connection,
        reference_version_name,
    )
    .await?;

    let ref_file_hash = find_file_hash_at_commit(&connection, &relative_path, &ref_commit_id)
        .await?
        .unwrap();

    if !allow_tools {
        return print_diff(&connection, &abs_path, &ref_file_hash).await;
    }

    let config = Config::read_config()?;

    match config.find_diff_command(&relative_path) {
        Some(mut external_command_vec) => {
            let ref_temp_file =
                download_temp_file(&connection, &workspace_root, &ref_file_hash).await?;
            let ref_path_str = ref_temp_file.to_str().unwrap();
            let local_file = abs_path.to_str().unwrap();
            for item in &mut external_command_vec[..] {
                *item = item.replace("%ref", ref_path_str);
                *item = item.replace("%local", local_file);
            }
            let output = Command::new(&external_command_vec[0])
                .args(&external_command_vec[1..])
                .output()
                .context(format!(
                    "Failed to execute external diff command: {:?}",
                    external_command_vec
                ))?;

            let mut out = std::io::stdout();
            out.write_all(&output.stdout).unwrap();
            out.flush().unwrap();

            let mut err = std::io::stderr();
            err.write_all(&output.stderr).unwrap();
            err.flush().unwrap();
        }
        None => {
            return print_diff(&connection, &abs_path, &ref_file_hash).await;
        }
    }

    Ok(())
}
