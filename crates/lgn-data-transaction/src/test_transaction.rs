/* TODO Re-enable after Loading/Handle refactor

use std::sync::Arc;

use generic_data::offline::TestEntity;
use lgn_content_store::{
    indexing::{empty_tree_id, SharedTreeIdentifier},
    Provider,
};
use lgn_data_build::DataBuildOptions;
use lgn_data_compiler::compiler_node::CompilerRegistryOptions;
use lgn_data_offline::{Project, ResourcePathName};
use lgn_data_runtime::{
AssetRegistryOptions, ResourceDescriptor, ResourceId, ResourceTypeAndId,
};
use lgn_data_runtime::{AssetRegistry, ResourcePathId};
use tokio::sync::Mutex;

use crate::SelectionManager;
use crate::{
    build_manager::BuildManager, ArrayOperation, CloneResourceOperation, CreateResourceOperation,
    DeleteResourceOperation, Error, RenameResourceOperation, Transaction, TransactionManager,
    UpdatePropertyOperation,
};

async fn validate_test_entity(
    res_id: ResourceTypeAndId,
    asset_registry: &AssetRegistry,
    callback: fn(test_entity: &TestEntity),
) {
    if let Ok(handle) = asset_registry.load_async::<TestEntity>(res_id).await {
        let test_entity = handle.get().unwrap();
        callback(&*test_entity);
    }
}

async fn test_array_insert_operation(
    resource_id: ResourceTypeAndId,
    transaction_manager: &mut TransactionManager,
) -> Result<(), Error> {
    // Add two entries to test_blob array
    let transaction = Transaction::new()
        .add_operation(ArrayOperation::insert_element(
            resource_id,
            "test_blob",
            Some(0),
            Some("255"),
        ))
        .add_operation(ArrayOperation::insert_element(
            resource_id,
            "test_blob",
            Some(1),
            Some("254"),
        ))
        .add_operation(ArrayOperation::insert_element(
            resource_id,
            "test_blob",
            Some(6),
            Some("253"),
        ));
    transaction_manager.commit_transaction(transaction).await?;
    validate_test_entity(
        resource_id,
        &transaction_manager.asset_registry,
        |test_entity| {
            assert_eq!(test_entity.test_blob, vec![255u8, 254u8, 0, 1, 2, 3, 253]);
        },
    )
    .await;

    // Undo transaction
    transaction_manager.undo_transaction().await?;
    validate_test_entity(
        resource_id,
        &transaction_manager.asset_registry,
        |test_entity| {
            assert_eq!(test_entity.test_blob, vec![0, 1, 2, 3]);
        },
    )
    .await;
    Ok(())
}

async fn test_array_delete_operation(
    resource_id: ResourceTypeAndId,
    transaction_manager: &mut TransactionManager,
) -> Result<(), Error> {
    // Add two entries to test_blob array
    let transaction = Transaction::new()
        .add_operation(ArrayOperation::delete_element(resource_id, "test_blob", 3))
        .add_operation(ArrayOperation::delete_element(resource_id, "test_blob", 1));
    transaction_manager.commit_transaction(transaction).await?;
    validate_test_entity(
        resource_id,
        &transaction_manager.asset_registry,
        |test_entity| {
            assert_eq!(test_entity.test_blob, vec![0, 2]);
        },
    )
    .await;

    // Undo transaction
    transaction_manager.undo_transaction().await?;
    validate_test_entity(
        resource_id,
        &transaction_manager.asset_registry,
        |test_entity| {
            assert_eq!(test_entity.test_blob, vec![0, 1, 2, 3]);
        },
    )
    .await;

    Ok(())
}

async fn test_array_reorder_operation(
    resource_id: ResourceTypeAndId,
    transaction_manager: &mut TransactionManager,
) -> Result<(), Error> {
    // Add two entries to test_blob array
    let transaction = Transaction::new()
        .add_operation(ArrayOperation::reorder_element(
            resource_id,
            "test_blob",
            0,
            1,
        ))
        .add_operation(ArrayOperation::reorder_element(
            resource_id,
            "test_blob",
            2,
            3,
        ));
    transaction_manager.commit_transaction(transaction).await?;
    validate_test_entity(
        resource_id,
        &transaction_manager.asset_registry,
        |test_entity| {
            assert_eq!(test_entity.test_blob, vec![1, 0, 3, 2]);
        },
    )
    .await;

    // Undo transaction
    transaction_manager.undo_transaction().await?;
    validate_test_entity(
        resource_id,
        &transaction_manager.asset_registry,
        |test_entity| {
            assert_eq!(test_entity.test_blob, vec![0, 1, 2, 3]);
        },
    )
    .await;
    Ok(())
}

#[tokio::test]
async fn test_transaction_system() -> Result<(), Error> {
    let project_dir = tempfile::tempdir().unwrap();
    let build_dir = project_dir.path().join("temp");
    std::fs::create_dir_all(&build_dir).unwrap();

    let source_control_content_provider = Arc::new(Provider::new_in_memory());
    let data_content_provider = Arc::new(Provider::new_in_memory());

    let project =
        Project::new_with_remote_mock(&project_dir, Arc::clone(&source_control_content_provider))
            .await
            .unwrap();

    let runtime_manifest_id =
        SharedTreeIdentifier::new(empty_tree_id(&data_content_provider).await.unwrap());
    let mut asset_registry = AssetRegistryOptions::new()
        .add_device_cas(
            Arc::clone(&data_content_provider),
            runtime_manifest_id.clone(),
        )
        .add_loader::<TestEntity>();
    generic_data::offline::register_types(&mut asset_registry);
    let asset_registry = asset_registry.create().await;

    let compilers =
        CompilerRegistryOptions::default().add_compiler(&lgn_compiler_testentity::COMPILER_INFO);

    let options = DataBuildOptions::new_with_sqlite_output(
        &build_dir,
        compilers,
        Arc::clone(&source_control_content_provider),
        Arc::clone(&data_content_provider),
    )
    .asset_registry(asset_registry.clone());

    let build_manager = BuildManager::new(options, &project, runtime_manifest_id.clone())
        .await
        .unwrap();

    let project = Arc::new(Mutex::new(project));

    {
        let mut transaction_manager = TransactionManager::new(
            project.clone(),
            asset_registry.clone(),
            build_manager,
            SelectionManager::create(),
        );
        let resource_path: ResourcePathName = "/entity/create_test77".into();

        let new_id = ResourceTypeAndId {
            kind: TestEntity::TYPE,
            id: ResourceId::new(),
        };

        let ref_resource_path: ResourcePathName = "/entity/create_reference".into();
        let ref_new_id = ResourceTypeAndId {
            kind: TestEntity::TYPE,
            id: ResourceId::new(),
        };

        let ref_path_id =
            ResourcePathId::from(ref_new_id).push(generic_data::runtime::TestEntity::TYPE);
        let ref_path_id = serde_json::to_value(ref_path_id).unwrap();

        // Create a new Resource, Edit some properties and Commit it
        let transaction = Transaction::new()
            .add_operation(CreateResourceOperation::new(
                new_id,
                resource_path.clone(),
                false,
                None,
            ))
            .add_operation(CreateResourceOperation::new(
                ref_new_id,
                ref_resource_path.clone(),
                false,
                None,
            ))
            .add_operation(UpdatePropertyOperation::new(
                new_id,
                &[("test_string", "\"Update1\"")],
            ))
            .add_operation(UpdatePropertyOperation::new(
                new_id,
                &[("test_bool", "false")],
            ))
            .add_operation(UpdatePropertyOperation::new(
                new_id,
                &[("test_position", "[1,2,3]")],
            ))
            .add_operation(UpdatePropertyOperation::new(
                new_id,
                &[("test_resource_path_option", ref_path_id.to_string())],
            ))
            .add_operation(ArrayOperation::insert_element(
                new_id,
                "test_resource_path_vec",
                None,
                Some(ref_path_id.to_string()),
            ))
            .add_operation(ArrayOperation::insert_element(
                new_id,
                "test_resource_path_vec",
                None,
                Some(ref_path_id.to_string()),
            ));
        transaction_manager.commit_transaction(transaction).await?;

        asset_registry.update();

        assert!(project.lock().await.exists_named(&resource_path).await);

        // Test Array Insert Operation
        test_array_insert_operation(new_id, &mut transaction_manager).await?;
        asset_registry.update();

        // Test Array Delete Operation
        test_array_delete_operation(new_id, &mut transaction_manager).await?;
        asset_registry.update();

        // Test Array Reorder Operation
        test_array_reorder_operation(new_id, &mut transaction_manager).await?;
        asset_registry.update();

        // Expected clone name
        let clone_name: ResourcePathName = "/entity/create_test78".into();
        let clone_id = ResourceTypeAndId {
            kind: TestEntity::TYPE,
            id: ResourceId::new(),
        };
        let transaction =
            Transaction::new().add_operation(CloneResourceOperation::new(new_id, clone_id, None));
        transaction_manager.commit_transaction(transaction).await?;
        asset_registry.update();
        assert!(project.lock().await.exists_named(&clone_name).await);
        assert!(project.lock().await.exists(clone_id).await);

        // Rename the clone
        let rename_new_name: ResourcePathName = "/entity/test_clone_rename".into();
        let transaction = Transaction::new().add_operation(RenameResourceOperation::new(
            clone_id,
            rename_new_name.clone(),
        ));
        transaction_manager.commit_transaction(transaction).await?;
        asset_registry.update();
        assert!(project.lock().await.exists_named(&rename_new_name).await);
        assert!(!project.lock().await.exists_named(&clone_name).await);

        // Undo Rename
        transaction_manager.undo_transaction().await?;
        asset_registry.update();
        assert!(!project.lock().await.exists_named(&rename_new_name).await);
        assert!(project.lock().await.exists_named(&clone_name).await);

        // Undo Clone
        transaction_manager.undo_transaction().await?;
        asset_registry.update();
        assert!(!project.lock().await.exists_named(&clone_name).await);
        assert!(!project.lock().await.exists(clone_id).await);

        // Delete the created Resource
        let transaction = Transaction::new().add_operation(DeleteResourceOperation::new(new_id));
        transaction_manager.commit_transaction(transaction).await?;
        asset_registry.update();
        assert!(!project.lock().await.exists_named(&resource_path).await);
        assert!(!project.lock().await.exists(new_id).await);

        // Undo delete
        transaction_manager.undo_transaction().await?;
        asset_registry.update();
        assert!(project.lock().await.exists_named(&resource_path).await);
        assert!(project.lock().await.exists(new_id).await);

        // Undo Create
        transaction_manager.undo_transaction().await?;
        asset_registry.update();
        assert!(!project.lock().await.exists_named(&resource_path).await);
        assert!(!project.lock().await.exists(new_id).await);

        // Redo Create
        transaction_manager.redo_transaction().await?;
        asset_registry.update();
        assert!(project.lock().await.exists_named(&resource_path).await);
        assert!(project.lock().await.exists(new_id).await);

        // Redo Delete
        transaction_manager.redo_transaction().await?;
        asset_registry.update();
        assert!(!project.lock().await.exists_named(&resource_path).await);
        assert!(!project.lock().await.exists(new_id).await);

        // Create Transaction with invalid edit
        let invalid_resource: ResourcePathName = "/entity/create_invalid.dc".into();
        let resource_path: ResourcePathName = "/entity/create_test.dc".into();
        let new_id = ResourceTypeAndId {
            kind: TestEntity::TYPE,
            id: ResourceId::new(),
        };
        let transaction = Transaction::new()
            .add_operation(CreateResourceOperation::new(
                new_id,
                resource_path,
                false,
                None,
            ))
            .add_operation(UpdatePropertyOperation::new(
                new_id,
                &[
                    ("test_string", "\"Update2\""),
                    ("test_bool", "false"),
                    ("INVALID_PROPERTY", "[1,2,3]"),
                ],
            ));

        assert!(
            transaction_manager
                .commit_transaction(transaction)
                .await
                .is_err(),
            "Transaction with invalid property update shouldn't succceed"
        );
        asset_registry.update();
        assert!(!project.lock().await.exists_named(&invalid_resource).await);

        // Test CreateResource with auto Name Increment
        for _ in 0..2 {
            let transaction = Transaction::new().add_operation(CreateResourceOperation::new(
                ResourceTypeAndId {
                    kind: TestEntity::TYPE,
                    id: ResourceId::new(),
                },
                "/entity/autoincrement1337".into(),
                true,
                None,
            ));
            transaction_manager.commit_transaction(transaction).await?;
        }
        asset_registry.update();
        assert!(
            project
                .lock()
                .await
                .exists_named(&"/entity/autoincrement1337".into())
                .await
        );
        assert!(
            project
                .lock()
                .await
                .exists_named(&"/entity/autoincrement1338".into())
                .await
        );

        drop(transaction_manager);
    }

    drop(asset_registry);
    drop(project);
    Ok(())
}
*/
