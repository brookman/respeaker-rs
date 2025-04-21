use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use clap::{command, Parser, Subcommand};
use eyre::eyre;
use eyre::Ok;
use eyre::Result;
use params::ParamKind;
use params::ParamState;
use recorder::record_respeaker_parameters;
use respeaker_device::ReSpeakerDevice;

use tracing::info;
use tracing::Level;
use ui::run_ui;

mod csv;
mod params;
mod recorder;
mod respeaker_device;
mod ui;

/// Unofficial CLI & UI for the Re-Speaker Mic Array v2.0
#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Arguments {
    #[command(subcommand)]
    command: Option<Command>,

    #[clap(short = 'i')]
    device_index: Option<usize>,
}

#[derive(Subcommand, Debug)]
#[clap(flatten_help = true)]
enum Command {
    /// List all available parameters and their current values (RW and RO).
    List,
    /// Read the value of a specific parameter.
    Read { param: ParamKind },
    /// Write the value of a specific parameter.
    Write { param: ParamKind, value: String },
    /// Perform a device reset.
    Reset,
    /// Continously record parameters to CSV file during the provided amount of seconds.
    /// The RW parameters are only read once at the start.
    Record {
        seconds: f32,
        csv_path: Option<PathBuf>,
    },
}

fn main() -> eyre::Result<()> {
    let args: Arguments = init()?;

    info!("Running unofficial ReSpeaker CLI with {args:?}");

    let shared_state = Arc::new(Mutex::new(ParamState {
        current_params: HashMap::new(),
    }));

    let mut device = ReSpeakerDevice::open(args.device_index, shared_state)?;

    if let Some(command) = args.command {
        match command {
            Command::List => {
                let list = device.list()?;
                info!("Parameters:\n{list}");
            }
            Command::Read { param } => {
                let value = device.read(&param)?;
                info!("\n{param:?}={value}");
            }
            Command::Write { param, value } => {
                let value = param.parse_value(&value)?;
                device.write(&param, &value)?;
            }
            Command::Reset => device.reset()?,
            Command::Record { seconds, csv_path } => {
                device.list()?; // cache rw params
                record_respeaker_parameters(seconds, csv_path, &device)?;
            }
        }
    } else {
        info!("Opening UI...");
        run_ui(device).map_err(|e| eyre!("UI error: {}", e))?;
    }

    Ok(())
}

fn init<T>() -> Result<T>
where
    T: Parser,
{
    let args = T::try_parse()?;
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .try_init()
        .map_err(|e| eyre!("Tracing init error: {e}"))?;
    Ok(args)
}
