use std::{
    fmt,
    sync::{Arc, Mutex},
};

use lgn_tracing::{
    dispatch::{flush_log_buffer, log_enabled, log_interop},
    event_sink::{EventSink, ProcessInfo},
    log_block::{LogBlock, LogMsgQueueAny, LogStream},
    log_events::LogDesc,
    metrics_block::{MetricsBlock, MetricsMsgQueueAny, MetricsStream},
    thread_block::{ThreadBlock, ThreadEventQueueAny, ThreadStream},
    Level,
};
use lgn_tracing_transit::HeterogeneousQueue;

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Startup(bool),
    Shutdown,
    LogEnabled(Level),
    Log(String),
    InitLogStream,
    ProcessLogBlock(usize),
    InitMetricsStream,
    ProcessMetricsBlock(usize),
    InitThreadStream,
    ProcessThreadBlock(usize),
}

pub type SharedState = Arc<Mutex<Option<State>>>;
pub struct DebugEventSink(SharedState);

impl DebugEventSink {
    pub fn new(state: SharedState) -> Self {
        Self(state)
    }
}

impl EventSink for DebugEventSink {
    fn on_startup(&self, process_info: ProcessInfo) {
        *self.0.lock().unwrap() = Some(State::Startup(!process_info.process_id.is_empty()));
    }

    fn on_shutdown(&self) {
        *self.0.lock().unwrap() = Some(State::Shutdown);
    }

    fn on_log_enabled(&self, level: Level, _: &str) -> bool {
        *self.0.lock().unwrap() = Some(State::LogEnabled(level));
        true
    }

    fn on_log(&self, _desc: &LogDesc, _time: i64, args: &fmt::Arguments<'_>) {
        *self.0.lock().unwrap() = Some(State::Log(args.to_string()));
    }

    fn on_init_log_stream(&self, _: &LogStream) {
        *self.0.lock().unwrap() = Some(State::InitLogStream);
    }

    fn on_process_log_block(&self, log_block: std::sync::Arc<LogBlock>) {
        use LogMsgQueueAny::*;
        for event in log_block.events.iter() {
            match event {
                LogStaticStrEvent(_evt) => {}
                LogStringEvent(_evt) => {}
                LogStaticStrInteropEvent(_evt) => {}
                LogStringInteropEvent(_evt) => {}
            }
        }
        *self.0.lock().unwrap() = Some(State::ProcessLogBlock(log_block.events.nb_objects()));
    }

    fn on_init_metrics_stream(&self, _: &MetricsStream) {
        *self.0.lock().unwrap() = Some(State::InitMetricsStream);
    }

    fn on_process_metrics_block(&self, metrics_block: std::sync::Arc<MetricsBlock>) {
        use MetricsMsgQueueAny::*;
        for event in metrics_block.events.iter() {
            match event {
                IntegerMetricEvent(_evt) => {}
                FloatMetricEvent(_evt) => {}
            }
        }
        *self.0.lock().unwrap() = Some(State::ProcessMetricsBlock(
            metrics_block.events.nb_objects(),
        ));
    }

    fn on_init_thread_stream(&self, _: &ThreadStream) {
        *self.0.lock().unwrap() = Some(State::InitThreadStream);
    }

    fn on_process_thread_block(&self, thread_block: std::sync::Arc<ThreadBlock>) {
        use ThreadEventQueueAny::*;
        for event in thread_block.events.iter() {
            match event {
                BeginThreadSpanEvent(_evt) => {}
                EndThreadSpanEvent(_evt) => {}
            }
        }
        *self.0.lock().unwrap() = Some(State::ProcessThreadBlock(thread_block.events.nb_objects()));
    }
}

pub struct LogDispatch;

impl log::Log for LogDispatch {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        let level = match metadata.level() {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warn,
            log::Level::Info => Level::Info,
            log::Level::Debug => Level::Debug,
            log::Level::Trace => Level::Trace,
        };
        log_enabled(metadata.target(), level)
    }

    fn log(&self, record: &log::Record<'_>) {
        let level = match record.level() {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warn,
            log::Level::Info => Level::Info,
            log::Level::Debug => Level::Debug,
            log::Level::Trace => Level::Trace,
        };
        let log_desc = LogDesc {
            level: level as u32,
            fmt_str: record.args().as_str().unwrap_or(""),
            target: record.module_path_static().unwrap_or("unknown"),
            module_path: record.module_path_static().unwrap_or("unknown"),
            file: record.file_static().unwrap_or("unknown"),
            line: record.line().unwrap_or(0),
        };
        log_interop(&log_desc, record.args());
    }
    fn flush(&self) {
        flush_log_buffer();
    }
}

#[macro_export]
macro_rules! expect_state {
    ($state:expr, $expected:expr) => {{
        let state = $state.lock().unwrap().take();
        assert_eq!(state, $expected)
    }};
    () => {};
}
