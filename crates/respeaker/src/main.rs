use clap::{command, Parser, Subcommand};
use eyre::eyre;
use eyre::Ok;
use eyre::Result;
use params::Access;
use params::Param;
use params::ParamConfig;
use params::ParseValue;
use params::Value;
use rusb::DeviceHandle;
use rusb::GlobalContext;

use strum::IntoEnumIterator;
use tabled::Table;
use tabled::Tabled;
use tracing::info;
use tracing::Level;
use ui::run_ui;
use usb::{open_device, read, reset, write};

mod params;
mod ui;
mod usb;

/// Unofficial CLI & UI for the ReSpeaker Mic Array v2.0
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

    let (device, interface) = open_device(args.device_index)?;

    if let Some(command) = args.command {
        match command {
            Command::List => {
                let list = list(&device)?;
                info!("Parameters:\n{list}");
            }
            Command::Read { param } => {
                let config = param.config();
                let value = read(&device, config)?;
                info!("\n{param:?}={value}");
            }
            Command::Write { param, value } => {
                write(&device, &param, &param.config().parse_value(&value)?)?;
            }
            Command::Reset => reset(&device, interface)?,
        }
    } else {
        info!("Opening UI...");
        run_ui(&device).map_err(|e| eyre!("UI error: {}", e))?;
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

#[derive(Tabled)]
struct TableRow {
    name: String,
    value: Value,
    t: String,
    access: String,
    range: String,
    description: String,
    values: String,
}

fn list(device_handle: &DeviceHandle<GlobalContext>) -> Result<String> {
    let mut rows = vec![];
    for p in Param::iter() {
        let config = p.config();
        let value = read(device_handle, config)?;
        match config {
            ParamConfig::IntMany(config) | ParamConfig::IntFew(config) => rows.push(TableRow {
                name: format!("{p:?}"),
                value,
                t: "int".to_string(),
                access: if config.access == Access::ReadOnly {
                    "ro"
                } else {
                    "rw"
                }
                .to_string(),
                range: format!("{}..{}", config.min, config.max),
                description: config.description.clone(),
                values: config.value_descriptions.join("\n"),
            }),
            ParamConfig::Float(config) => rows.push(TableRow {
                name: format!("{p:?}"),
                value,
                t: "float".to_string(),
                access: if config.access == Access::ReadOnly {
                    "ro"
                } else {
                    "rw"
                }
                .to_string(),
                range: format!("{}..{}", config.min, config.max),
                description: config.description.clone(),
                values: config.value_descriptions.join("\n"),
            }),
        }
    }
    Ok(Table::new(rows).to_string())
}
