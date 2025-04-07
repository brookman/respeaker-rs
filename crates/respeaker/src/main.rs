use clap::{Parser, Subcommand, command};
use eyre::Context;
use eyre::Ok;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
use params::Access;
use params::Param;
use params::ParamConfig;
use rusb::DeviceHandle;
use rusb::GlobalContext;
use std::time::Duration;
use strum::IntoEnumIterator;
use tabled::Table;
use tabled::Tabled;
use tracing::Level;
use tracing::info;

mod params;

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
}

fn main() -> eyre::Result<()> {
    let args: Arguments = init()?;

    info!("Running unofficial ReSpeaker CLI with {args:?}");

    let device = open_device(args.device_index)?;

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
            Command::Write { param, value } => write(&device, &param, &value)?,
        }
    } else {
        info!("Opening UI...");
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

fn open_device(device_index: Option<usize>) -> Result<DeviceHandle<GlobalContext>> {
    const VENDOR_ID: u16 = 0x2886;
    const PRODUCT_ID: u16 = 0x0018;

    info!("Searching for ReSpeaker Mic Array v2.0 device...");

    let mut devices = vec![];

    for device in rusb::devices()?.iter() {
        let device_desc = device.device_descriptor()?;

        if device_desc.vendor_id() == VENDOR_ID && device_desc.product_id() == PRODUCT_ID {
            info!(
                "Found: Bus {:03} Device {:03} ID {:04x}:{:04x}",
                device.bus_number(),
                device.address(),
                device_desc.vendor_id(),
                device_desc.product_id()
            );
            devices.push(device);
        }
    }
    if let Some(i) = device_index {
        if let Some(d) = devices.get(i) {
            return Ok(d.open()?);
        }
        bail!(
            "Device index (-i argument) out of range. Index was {i} but {} devices found.",
            devices.len()
        );
    }
    if devices.len() == 1 {
        let handle = devices[0].open()?;
        return Ok(handle);
    }
    if devices.len() > 1 {
        bail!("Multiple devices found. Specify the a device index with -i.")
    }

    bail!("No devices found")
}

#[derive(Tabled)]
struct TableRow {
    name: String,
    value: String,
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
            ParamConfig::IntN(config)
            | ParamConfig::Int2(config)
            | ParamConfig::Int3(config)
            | ParamConfig::Int4(config) => rows.push(TableRow {
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

fn read(device_handle: &DeviceHandle<GlobalContext>, param_config: &ParamConfig) -> Result<String> {
    let (is_int, id, cmd) = match param_config {
        ParamConfig::IntN(config)
        | ParamConfig::Int2(config)
        | ParamConfig::Int3(config)
        | ParamConfig::Int4(config) => (true, config.id, config.cmd),
        ParamConfig::Float(config) => (false, config.id, config.cmd),
    };

    let mut cmd = 0x80 | cmd;
    if is_int {
        cmd |= 0x40;
    }

    let mut buffer = [0u8; 8];

    let request_type = rusb::request_type(
        rusb::Direction::In,
        rusb::RequestType::Vendor,
        rusb::Recipient::Device,
    );

    device_handle.read_control(
        request_type,
        0,
        cmd,
        id,
        &mut buffer,
        Duration::from_secs(3),
    )?;
    let response = (
        i32::from_le_bytes(buffer[0..4].try_into()?),
        i32::from_le_bytes(buffer[4..8].try_into()?),
    );

    let result = if is_int {
        format!("{}", response.0)
    } else {
        #[allow(clippy::cast_possible_truncation)]
        let float = (f64::from(response.0) * f64::from(response.1).exp2()) as f32;
        format!("{float}")
    };

    Ok(result)
}

fn write(device_handle: &DeviceHandle<GlobalContext>, param: &Param, value: &str) -> Result<()> {
    let config = param.config();

    let (id, cmd, access) = match config {
        ParamConfig::IntN(config)
        | ParamConfig::Int2(config)
        | ParamConfig::Int3(config)
        | ParamConfig::Int4(config) => (config.id, config.cmd, config.access),
        ParamConfig::Float(config) => (config.id, config.cmd, config.access),
    };

    if access == Access::ReadOnly {
        bail!("Parameter {:?} is read-only", param);
    }

    let (cmd_bytes, value_bytes, type_bytes) = match config {
        ParamConfig::IntN(config)
        | ParamConfig::Int2(config)
        | ParamConfig::Int3(config)
        | ParamConfig::Int4(config) => {
            let value = value
                .parse::<i32>()
                .context("Could not parse value as int")?;

            if value < config.min || value > config.max {
                bail!(
                    "Value {value} is not in range {}..{}",
                    config.min,
                    config.max
                );
            }
            (
                i32::from(cmd).to_le_bytes(),
                value.to_le_bytes(),
                1i32.to_le_bytes(),
            )
        }
        ParamConfig::Float(config) => {
            let value = value
                .parse::<f32>()
                .context("Could not parse value as float")?;

            if value < config.min || value > config.max {
                bail!(
                    "Value {value} is not in range {}..{}",
                    config.min,
                    config.max
                );
            }
            (
                i32::from(cmd).to_le_bytes(),
                value.to_le_bytes(),
                0i32.to_le_bytes(),
            )
        }
    };

    let mut payload = Vec::with_capacity(12);
    payload.extend_from_slice(&cmd_bytes);
    payload.extend_from_slice(&value_bytes);
    payload.extend_from_slice(&type_bytes);

    let request_type = rusb::request_type(
        rusb::Direction::Out,
        rusb::RequestType::Vendor,
        rusb::Recipient::Device,
    );

    device_handle.write_control(request_type, 0, 0, id, &payload, Duration::from_secs(3))?;

    info!("Wrote value {value} to param {:?} successfully", param);

    Ok(())
}
