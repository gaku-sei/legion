#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use lgn_content_store::Provider;
    use lgn_data_compiler::compiler_node::CompilerRegistryOptions;
    use lgn_data_offline::resource::{Project, ResourcePathName};
    use lgn_data_runtime::{AssetRegistry, AssetRegistryOptions, ResourcePathId};
    use lgn_source_control::LocalRepositoryIndex;
    use tempfile::TempDir;

    use crate::DataBuildOptions;

    pub(crate) async fn setup_dir(
        work_dir: &TempDir,
    ) -> (
        PathBuf,
        PathBuf,
        LocalRepositoryIndex,
        Arc<Provider>,
        Arc<Provider>,
    ) {
        let project_dir = work_dir.path();
        let output_dir = project_dir.join("temp");
        std::fs::create_dir_all(&output_dir).unwrap();

        let repository_index = LocalRepositoryIndex::new(project_dir.join("remote"))
            .await
            .unwrap();
        let source_control_content_provider = Arc::new(Provider::new_in_memory());
        let data_content_provider = Arc::new(Provider::new_in_memory());

        (
            project_dir.to_owned(),
            output_dir,
            repository_index,
            source_control_content_provider,
            data_content_provider,
        )
    }

    async fn setup_registry() -> Arc<AssetRegistry> {
        let mut options = AssetRegistryOptions::new();
        refs_resource::TestResource::register_type(&mut options);
        options.create().await
    }

    #[tokio::test]
    async fn no_dependencies() {
        let work_dir = tempfile::tempdir().unwrap();
        let (
            project_dir,
            output_dir,
            repository_index,
            source_control_content_provider,
            data_content_provider,
        ) = setup_dir(&work_dir).await;
        let resources = setup_registry().await;

        let resource = {
            let mut project = Project::create_with_remote_mock(
                &project_dir,
                Arc::clone(&source_control_content_provider),
            )
            .await
            .expect("failed to create a project");
            let id = project
                .add_resource(
                    ResourcePathName::new("resource"),
                    &resources
                        .new_resource::<refs_resource::TestResource>()
                        .unwrap(),
                    &resources,
                )
                .await
                .unwrap();
            ResourcePathId::from(id)
        };

        let (mut build, project) = DataBuildOptions::new_with_sqlite_output(
            &output_dir,
            CompilerRegistryOptions::default(),
            data_content_provider,
        )
        .create_with_project(
            project_dir,
            repository_index,
            source_control_content_provider,
        )
        .await
        .expect("data build");

        build.source_pull(&project).await.unwrap();

        let source_index = build.source_index.current().unwrap();

        assert!(source_index.find_dependencies(&resource).is_some());
        assert_eq!(source_index.find_dependencies(&resource).unwrap().len(), 0);
    }

    #[tokio::test]
    async fn with_dependency() {
        let work_dir = tempfile::tempdir().unwrap();
        let (
            project_dir,
            output_dir,
            repository_index,
            source_control_content_provider,
            data_content_provider,
        ) = setup_dir(&work_dir).await;
        let resources = setup_registry().await;

        let (child_id, parent_id) = {
            let mut project = Project::create_with_remote_mock(
                &project_dir,
                Arc::clone(&source_control_content_provider),
            )
            .await
            .expect("failed to create a project");
            let child_id = project
                .add_resource(
                    ResourcePathName::new("child"),
                    &resources
                        .new_resource::<refs_resource::TestResource>()
                        .unwrap(),
                    &resources,
                )
                .await
                .unwrap();

            let parent_handle = {
                let handle = resources
                    .new_resource::<refs_resource::TestResource>()
                    .unwrap();
                let mut edit = resources.edit(&handle).unwrap();
                edit.build_deps.push(ResourcePathId::from(child_id));
                resources.commit(edit);
                handle
            };
            let parent_id = project
                .add_resource(ResourcePathName::new("parent"), &parent_handle, &resources)
                .await
                .unwrap();
            (
                ResourcePathId::from(child_id),
                ResourcePathId::from(parent_id),
            )
        };

        let (mut build, project) = DataBuildOptions::new_with_sqlite_output(
            &output_dir,
            CompilerRegistryOptions::default(),
            data_content_provider,
        )
        .create_with_project(
            project_dir,
            repository_index,
            source_control_content_provider,
        )
        .await
        .expect("data build");

        build.source_pull(&project).await.unwrap();

        let source_index = build.source_index.current().unwrap();

        let child_deps = source_index
            .find_dependencies(&child_id)
            .expect("zero deps");
        let parent_deps = source_index.find_dependencies(&parent_id).expect("one dep");

        assert_eq!(child_deps.len(), 0);
        assert_eq!(parent_deps.len(), 1);
    }
}
