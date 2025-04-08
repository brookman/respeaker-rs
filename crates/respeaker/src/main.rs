use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use clap::{Parser, Subcommand, command};
use eyre::Ok;
use eyre::Result;
use eyre::eyre;
use params::Param;
use params::ParamState;
use params::ParseValue;
use recorder::record_audio;
use respeaker_device::ReSpeakerDevice;

use tracing::Level;
use tracing::info;
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
    /// List all available parameters and their current values (RW and RO)
    List,
    /// Read the value of a specific parameter
    Read { param: Param },
    /// Write the value of a specific parameter
    Write { param: Param, value: String },
    /// Perform a device reset
    Reset,
    /// Record audio for the provided amount of seconds
    Record {
        seconds: f32,
        wav_path: Option<PathBuf>,
        /// Override the input device (mic) index. If not set the first Re-Speaker device will be used.
        #[clap(short = 'm')]
        mic_index: Option<usize>,
    },
}

fn main() -> eyre::Result<()> {
    let args: Arguments = init()?;

    info!("Running unofficial ReSpeaker CLI with {args:?}");

    let shared_state = Arc::new(Mutex::new(ParamState {
        current_params: HashMap::new(),
    }));

    let mut device = ReSpeakerDevice::open(args.device_index, shared_state.clone())?;

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
                let value = param.config().parse_value(&value)?;
                device.write(&param, &value)?;
            }
            Command::Reset => device.reset()?,
            Command::Record {
                seconds,
                wav_path,
                mic_index,
            } => {
                device.list()?; // read all params into cache initially
                record_audio(seconds, wav_path, mic_index, device)?;
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
