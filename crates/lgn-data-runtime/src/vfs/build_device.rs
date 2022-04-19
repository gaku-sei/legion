use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use std::{
    io,
    path::{Path, PathBuf},
};

use lgn_content_store::Provider;
use lgn_tracing::info;

use super::Device;
use crate::AssetRegistryReader;
use crate::{manifest::Manifest, ResourceTypeAndId};

/// Storage device that builds resources on demand. Resources are accessed
/// through a manifest access table.
pub(crate) struct BuildDevice {
    manifest: Manifest,
    provider: Arc<Provider>,
    databuild_bin: PathBuf,
    output_db_addr: String,
    project: PathBuf,
    force_recompile: bool,
}

impl BuildDevice {
    pub(crate) fn new(
        manifest: Manifest,
        provider: Arc<Provider>,
        build_bin: impl AsRef<Path>,
        output_db_addr: String,
        project: impl AsRef<Path>,
        force_recompile: bool,
    ) -> Self {
        Self {
            manifest,
            provider,
            databuild_bin: build_bin.as_ref().to_owned(),
            output_db_addr,
            project: project.as_ref().to_owned(),
            force_recompile,
        }
    }
}

#[async_trait]
impl Device for BuildDevice {
    async fn load(&self, type_id: ResourceTypeAndId) -> Option<Vec<u8>> {
        if self.force_recompile {
            self.reload(type_id).await
        } else {
            let checksum = self.manifest.find(type_id)?;
            let content = self.provider.read(&checksum).await.ok()?;
            Some(content)
        }
    }

    async fn get_reader(&self, type_id: ResourceTypeAndId) -> Option<AssetRegistryReader> {
        if self.force_recompile {
            self.manifest.extend(self.build_resource(type_id).ok()?);
        }
        let checksum = &self.manifest.find(type_id)?;
        let reader = self.provider.get_reader(checksum).await.ok()?;

        Some(Box::pin(reader) as AssetRegistryReader)
    }

    async fn reload(&self, type_id: ResourceTypeAndId) -> Option<Vec<u8>> {
        let output = self.build_resource(type_id).ok()?;
        self.manifest.extend(output);

        let checksum = self.manifest.find(type_id)?;
        let content = self.provider.read(&checksum).await.ok()?;
        Some(content)
    }
}

impl BuildDevice {
    fn build_resource(&self, resource_id: ResourceTypeAndId) -> io::Result<Manifest> {
        let mut command = build_command(
            &self.databuild_bin,
            resource_id,
            &self.output_db_addr,
            &self.project,
        );

        info!("Running DataBuild for ResourceId: {}", resource_id);
        info!("{:?}", command);
        let start = Instant::now();
        let output = command.output()?;
        info!("{:?}", output);

        info!(
            "{} DataBuild for Resource: {} processed in {:?}",
            if output.status.success() {
                "Succeeded"
            } else {
                "Failed"
            },
            resource_id,
            start.elapsed(),
        );

        if !output.status.success() {
            eprintln!(
                "{:?}",
                std::str::from_utf8(&output.stdout).expect("valid utf8")
            );
            eprintln!(
                "{:?}",
                std::str::from_utf8(&output.stderr).expect("valid utf8")
            );

            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Data Build Failed: '{}'",
                    std::str::from_utf8(&output.stderr).expect("valid utf8")
                ),
            ));
        }

        let manifest: Manifest = serde_json::from_slice(&output.stdout).map_err(|_e| {
            std::io::Error::new(io::ErrorKind::InvalidData, "Failed to read manifest")
        })?;

        Ok(manifest)
    }
}

fn build_command(
    databuild_path: impl AsRef<Path>,
    resource_id: ResourceTypeAndId,
    output_db_addr: &str,
    project: impl AsRef<Path>,
) -> std::process::Command {
    let target = "game";
    let platform = "windows";
    let locale = "en";
    let mut command = std::process::Command::new(databuild_path.as_ref());
    command.arg("compile");
    command.arg(resource_id.to_string());
    command.arg("--rt");
    command.arg(format!("--target={}", target));
    command.arg(format!("--platform={}", platform));
    command.arg(format!("--locale={}", locale));
    command.arg(format!("--output={}", output_db_addr));
    command.arg(format!("--project={}", project.as_ref().to_str().unwrap()));
    command
}
