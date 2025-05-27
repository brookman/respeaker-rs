use std::{
    f32,
    fs::create_dir,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
        create_dir(dir)?;
    }
    let creation_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let csv_path =
        csv_path.unwrap_or_else(|| PathBuf::from(format!("./recordings/{creation_time}.csv")));
    let mut csv_writer = CsvWriter::new(&csv_path)?;

    let start = Instant::now();
    while running.load(Ordering::SeqCst)
        && start.elapsed().as_secs_f32() <= seconds_to_record.unwrap_or(f32::INFINITY)
    {
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
        csv_writer.write_row(start.elapsed().as_secs_f32(), &values)?;

        thread::sleep(Duration::from_millis(10));
    }

    drop(csv_writer);

    info!("Recording done");

    Ok(())
}
