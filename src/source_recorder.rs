use serde_json::json;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic::Location;
use std::sync::Mutex;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub fn record(stage: &'static str, source: &str, caller: &'static Location<'static>) {
    let Ok(output_path) = env::var("FTML_SOURCE_RECORD_PATH") else {
        return;
    };
    if output_path.is_empty() {
        return;
    }

    let thread = std::thread::current();
    let record = json!({
        "schema": "ftml.test_source_record.v1",
        "stage": stage,
        "test_name": thread.name(),
        "caller": {
            "file": caller.file(),
            "line": caller.line(),
            "column": caller.column(),
        },
        "source": source,
    });
    let mut line =
        serde_json::to_vec(&record).expect("test source record must serialize");
    line.push(b'\n');

    let _guard = WRITE_LOCK
        .lock()
        .expect("test source recorder lock poisoned");
    let mut output = OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path)
        .expect("test source record output must open");
    output
        .write_all(&line)
        .expect("test source record must write");
}
