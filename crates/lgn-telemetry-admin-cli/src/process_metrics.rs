use std::sync::Arc;

use anyhow::Result;
use lgn_analytics::prelude::*;
use lgn_blob_storage::BlobStorage;
use lgn_tracing_transit::prelude::*;

pub async fn print_process_metrics(
    connection: &mut sqlx::AnyConnection,
    blob_storage: Arc<dyn BlobStorage>,
    process_id: &str,
) -> Result<()> {
    for_each_process_metric(connection, blob_storage, process_id, |obj| {
        let desc = obj.get::<Arc<Object>>("desc").unwrap();
        let name = desc.get::<Arc<String>>("name").unwrap();
        let unit = desc.get::<Arc<String>>("unit").unwrap();
        let time = obj.get::<u64>("time").unwrap();
        if let Ok(int_value) = obj.get::<u64>("value") {
            println!("{} {} ({}) : {}", time, name, unit, int_value);
        } else if let Ok(float_value) = obj.get::<f64>("value") {
            println!("{} {} ({}) : {}", time, name, unit, float_value);
        }
    })
    .await?;
    Ok(())
}
