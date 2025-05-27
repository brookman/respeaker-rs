use chrono::Local;
use std::{
    f32, fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use eyre::Ok;
use tracing::info;

use crate::{csv::CsvWriter, respeaker_device::ReSpeakerDevice};

pub fn record_respeaker_parameters(
    seconds_to_record: Option<f32>,
    csv_path: Option<PathBuf>,
    device: &ReSpeakerDevice,
    running: &Arc<AtomicBool>,
) -> eyre::Result<()> {
    let dir = PathBuf::from("./recordings");
    if csv_path.is_none() && !dir.exists() {
        fs::create_dir(dir)?;
    }

    let start = Instant::now();

    let csv_path = csv_path.unwrap_or_else(|| {
        let timetamp = iso8601();
        let timestap_save = timetamp.replace(':', "_");
        PathBuf::from(format!("./recordings/{timestap_save}.csv"))
    });
    let mut csv_writer = CsvWriter::new(&csv_path)?;

    while running.load(Ordering::SeqCst)
        && start.elapsed().as_secs_f32() <= seconds_to_record.unwrap_or(f32::INFINITY)
    {
        let before = iso8601();
        device.read_ro()?; // update readonly values
        let values = {
            let params = device
                .params()
                .lock()
                .expect("Lock failed")
                .current_params
                .clone();
            params
        };
        let after = iso8601();
        csv_writer.write_row(&before, &after, &values)?;

        thread::sleep(Duration::from_millis(10));
    }

    drop(csv_writer);

    info!("Recording done");

    Ok(())
}

fn iso8601() -> String {
    let dt = Local::now();
    format!("{}", dt.format("%+"))
}
