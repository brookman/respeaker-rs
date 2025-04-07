use clap::{Parser, Subcommand, command};
use eyre::Ok;
use eyre::Result;
use eyre::eyre;
use params::Param;
use params::ParseValue;
use respeaker_device::ReSpeakerDevice;

use tracing::Level;
use tracing::info;
use ui::run_ui;

mod params;
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
}

fn main() -> eyre::Result<()> {
    let args: Arguments = init()?;

    info!("Running unofficial ReSpeaker CLI with {args:?}");

    let mut device = ReSpeakerDevice::open(args.device_index)?;

    if let Some(command) = args.command {
        match command {
            Command::List => {
                let list = device.list()?;
                info!("Parameters:\n{list}");
            }
            Command::Read { param } => {
                let config = param.config();
                let value = device.read(config)?;
                info!("\n{param:?}={value}");
            }
            Command::Write { param, value } => {
                let value = param.config().parse_value(&value)?;
                device.write(&param, &value)?;
            }
            Command::Reset => device.reset()?,
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
