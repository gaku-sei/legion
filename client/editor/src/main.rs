//! Editor client executable

// BEGIN - Legion Labs lints v0.5
// do not change or add/remove here, but one can add exceptions after this section
#![deny(unsafe_code)]
#![warn(future_incompatible, nonstandard_style, rust_2018_idioms)]
// Rustdoc lints
#![warn(
    rustdoc::broken_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::private_intra_doc_links
)]
// Clippy pedantic lints, treat all as warnings by default, add exceptions in allow list
#![warn(clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::unreadable_literal,
    clippy::unseparated_literal_suffix
)]
// Clippy nursery lints, still under development
#![warn(
    clippy::debug_assert_with_mut_call,
    clippy::disallowed_method,
    clippy::disallowed_type,
    clippy::fallible_impl_from,
    clippy::imprecise_flops,
    clippy::mutex_integer,
    clippy::path_buf_push_overwrite,
    clippy::string_lit_as_bytes,
    clippy::use_self,
    clippy::useless_transmute
)]
// Clippy restriction lints, usually not considered bad, but useful in specific cases
#![warn(
    clippy::dbg_macro,
    clippy::exit,
    clippy::float_cmp_const,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_to_string,
    clippy::todo,
    clippy::unimplemented,
    clippy::verbose_file_reads
)]
// END - Legion Labs standard lints v0.5
// crate-specific exceptions:
#![allow()]
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use async_std::future::timeout;
use clap::Arg;
use legion_app::prelude::*;
use legion_async::AsyncPlugin;
use legion_streaming_proto::{streamer_client::StreamerClient, InitializeStreamRequest};
use legion_tauri::{legion_tauri_command, TauriPlugin, TauriPluginSettings};
use legion_telemetry::prelude::*;
use std::{error::Error, time::Duration};

struct Config {
    server_addr: String,
}

impl Config {
    fn new(args: &clap::ArgMatches<'_>) -> anyhow::Result<Self> {
        Ok(Self {
            server_addr: args
                .value_of("server-addr")
                .unwrap_or("http://[::1]:50051")
                .parse()?,
        })
    }

    fn new_from_environment() -> anyhow::Result<Self> {
        let args = clap::App::new("Legion Labs editor")
            .author(clap::crate_authors!())
            .version(clap::crate_version!())
            .about("Legion Labs editor.")
            .arg(
                Arg::with_name("server-addr")
                    .long("server-addr")
                    .takes_value(true)
                    .help("The address of the editor server to connect to"),
            )
            .get_matches();

        Self::new(&args)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let _telemetry_guard = TelemetrySystemGuard::new(None);
    let _telemetry_thread_guard = TelemetryThreadGuard::new();
    let config = Config::new_from_environment()?;
    let builder =
        tauri::Builder::default()
            .manage(config)
            .invoke_handler(tauri::generate_handler![
                initialize_stream,
                on_video_close,
                on_video_chunk_received
            ]);

    App::new()
        .insert_non_send_resource(TauriPluginSettings::new(builder))
        .add_plugin(TauriPlugin::new(tauri::generate_context!()))
        .add_plugin(AsyncPlugin {})
        .run();
    Ok(())
}

#[tauri::command]
fn on_video_close() {
    flush_log_buffer();
    flush_metrics_buffer();
}

#[tauri::command]
fn on_video_chunk_received(chunk_header: &str) {
    static CHUNK_INDEX_IN_FRAME_METRIC: MetricDesc = MetricDesc {
        name: "Chunk Index in Frame",
        unit: "",
    };

    match json::parse(chunk_header) {
        Ok(header) => match header["chunk_index_in_frame"].as_i64() {
            Some(chunk_index) => {
                record_int_metric(&CHUNK_INDEX_IN_FRAME_METRIC, chunk_index as u64);
            }
            None => {
                log::error!("chunk_index_in_frame not found in chunk header");
            }
        },
        Err(e) => {
            log::error!("Error parsing chunk header: {}", e);
        }
    }
}

#[legion_tauri_command]
async fn initialize_stream(
    config: tauri::State<'_, Config>,
    rtc_session_description: String,
) -> anyhow::Result<String> {
    let mut client = timeout(
        Duration::from_secs(3),
        StreamerClient::connect(config.server_addr.clone()),
    )
    .await??;

    let rtc_session_description = base64::decode(rtc_session_description)?;
    let request = tonic::Request::new(InitializeStreamRequest {
        rtc_session_description,
    });

    let response = client.initialize_stream(request).await?.into_inner();

    if response.error.is_empty() {
        Ok(base64::encode(response.rtc_session_description))
    } else {
        Err(anyhow::format_err!("{}", response.error))
    }
}
