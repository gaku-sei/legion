use std::{fmt, sync::Arc};

use crate::{
    log_block::{LogBlock, LogStream},
    log_events::LogDesc,
    metrics_block::{MetricsBlock, MetricsStream},
    thread_block::{ThreadBlock, ThreadStream},
    Level,
};

#[derive(Debug)]
pub struct ProcessInfo {
    pub process_id: String,
    pub exe: String,
    pub username: String,
    pub realname: String,
    pub computer: String,
    pub distro: String,
    pub cpu_brand: String,
    pub tsc_frequency: u64,
    /// RFC 3339
    pub start_time: String,
    pub start_ticks: i64,
    pub parent_process_id: String,
}

pub trait EventSink {
    fn on_startup(&self, process_info: ProcessInfo);
    fn on_shutdown(&self);

    fn on_log_enabled(&self, level: Level, target: &str) -> bool;
    fn on_log(&self, desc: &LogDesc, time: i64, args: &fmt::Arguments<'_>);
    fn on_init_log_stream(&self, log_stream: &LogStream);
    fn on_process_log_block(&self, log_block: Arc<LogBlock>);

    fn on_init_metrics_stream(&self, metrics_stream: &MetricsStream);
    fn on_process_metrics_block(&self, metrics_block: Arc<MetricsBlock>);

    fn on_init_thread_stream(&self, thread_stream: &ThreadStream);
    fn on_process_thread_block(&self, thread_block: Arc<ThreadBlock>);
}

pub struct NullEventSink {}
impl EventSink for NullEventSink {
    fn on_startup(&self, _: ProcessInfo) {}
    fn on_shutdown(&self) {}

    fn on_log_enabled(&self, _: Level, _: &str) -> bool {
        false
    }
    fn on_log(&self, _: &LogDesc, _: i64, _: &fmt::Arguments<'_>) {}
    fn on_init_log_stream(&self, _: &LogStream) {}
    fn on_process_log_block(&self, _: Arc<LogBlock>) {}

    fn on_init_metrics_stream(&self, _: &MetricsStream) {}
    fn on_process_metrics_block(&self, _: Arc<MetricsBlock>) {}

    fn on_init_thread_stream(&self, _: &ThreadStream) {}
    fn on_process_thread_block(&self, _: Arc<ThreadBlock>) {}
}
