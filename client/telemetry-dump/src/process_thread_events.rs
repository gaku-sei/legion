use analytics::*;
use anyhow::*;
use std::path::Path;
use transit::*;

pub async fn print_process_thread_events(
    connection: &mut sqlx::AnyConnection,
    data_path: &Path,
    process_id: &str,
) -> Result<()> {
    for stream in find_process_thread_streams(connection, process_id).await? {
        for block in find_stream_blocks(connection, &stream.stream_id).await? {
            let payload = fetch_block_payload(connection, data_path, &block.block_id).await?;
            parse_block(&stream, &payload, |val| {
                if let Value::Object(obj) = val {
                    let time = obj.get::<u64>("time").unwrap();
                    let scope = obj.get::<Object>("scope").unwrap();
                    let name = scope.get::<String>("name").unwrap();
                    let filename = scope.get::<String>("filename").unwrap();
                    let line = scope.get::<u32>("line").unwrap();
                    println!("{} {} {} {}:{}", time, obj.type_name, name, filename, line);
                }
            })?;
        }
    }
    Ok(())
}
