use clap::{Parser, Subcommand, command};
use eyre::Context;
use eyre::Ok;
use eyre::Result;
use eyre::bail;
use eyre::eyre;
use params::Access;
use params::Param;
use params::ParamConfig;
use params::ParseValue;
use params::Value;
use rusb::Device;
use rusb::DeviceHandle;
use rusb::GlobalContext;
use std::time::Duration;
use std::time::Instant;
use strum::IntoEnumIterator;
use tabled::Table;
use tabled::Tabled;
use tracing::Level;
use tracing::info;
use ui::run_ui;

mod params;
mod ui;

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

const TIMEOUT: Duration = Duration::from_secs(2);

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

fn open_device(device_index: Option<usize>) -> Result<(DeviceHandle<GlobalContext>, u8)> {
    fn open(device: &Device<GlobalContext>) -> Result<(DeviceHandle<GlobalContext>, u8)> {
        let handle = device.open()?;

        let config_desc = device.active_config_descriptor()?;
        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                if interface_desc.class_code() == 0xfe && interface_desc.sub_class_code() == 0x01 {
                    let iface_num = interface_desc.interface_number();
                    return Ok((handle, iface_num));
                }
            }
        }
        bail!("Could not find correct interface")
    }

    const VENDOR_ID: u16 = 0x2886;
    const PRODUCT_ID: u16 = 0x0018;

    info!("Searching for ReSpeaker Mic Array v2.0 device...");

    let mut devices = vec![];

    for device in rusb::devices()?.iter() {
        let device_desc = device.device_descriptor()?;

        if device_desc.vendor_id() == VENDOR_ID && device_desc.product_id() == PRODUCT_ID {
            info!(
                "Found: Bus {:03} Device {:03} ID {:04x}:{:04x}, speed: {:?}",
                device.bus_number(),
                device.address(),
                device_desc.vendor_id(),
                device_desc.product_id(),
                device.speed()
            );
            devices.push(device);
        }
    }
    if let Some(i) = device_index {
        if let Some(d) = devices.get(i) {
            return open(d);
        }
        bail!(
            "Device index (-i argument) out of range. Index was {i} but {} devices found.",
            devices.len()
        );
    }
    if devices.len() == 1 {
        return open(&devices[0]);
    }
    if devices.len() > 1 {
        bail!("Multiple devices found. Specify the a device index with -i.")
    }

    bail!("No devices found")
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

fn read(device_handle: &DeviceHandle<GlobalContext>, param_config: &ParamConfig) -> Result<Value> {
    let start = Instant::now();
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

    device_handle.read_control(request_type, 0, cmd, id, &mut buffer, TIMEOUT)?;
    let response = (
        i32::from_le_bytes(buffer[0..4].try_into()?),
        i32::from_le_bytes(buffer[4..8].try_into()?),
    );
    info!("Read parameter in {:?}", start.elapsed());

    if is_int {
        if let ParamConfig::IntN(config)
        | ParamConfig::Int2(config)
        | ParamConfig::Int3(config)
        | ParamConfig::Int4(config) = param_config
        {
            return Ok(Value::Int(config.clone(), response.0));
        }
        unreachable!();
    } else {
        #[allow(clippy::cast_possible_truncation)]
        let float = (f64::from(response.0) * f64::from(response.1).exp2()) as f32;

        if let ParamConfig::Float(config) = param_config {
            return Ok(Value::Float(config.clone(), float));
        }
        unreachable!();
    }
}

fn write(device_handle: &DeviceHandle<GlobalContext>, param: &Param, value: &Value) -> Result<()> {
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
            let value = match value {
                Value::Int(_, i) => *i,
                Value::Float(_, _) => bail!("Value must be of type int"),
            };

            if value < config.min || value > config.max {
                bail!(
                    "Value {value} is not in range {}..={}",
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
            let value = match value {
                Value::Int(_, _) => bail!("Value must be of type float"),
                Value::Float(_, f) => *f,
            };

            if value < config.min || value > config.max {
                bail!(
                    "Value {value} is not in range {}..={}",
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

    device_handle.write_control(request_type, 0, 0, id, &payload, TIMEOUT)?;

    info!("Wrote value {value} to param {:?} successfully", param);

    Ok(())
}

fn reset(device_handle: &DeviceHandle<GlobalContext>, inteface: u8) -> Result<()> {
    const XMOS_DFU_RESETDEVICE: u8 = 0xF0;
    //const XMOS_DFU_REVERTFACTORY: u8 = 0xf1;

    let request_type = rusb::request_type(
        rusb::Direction::Out,
        rusb::RequestType::Class,
        rusb::Recipient::Interface,
    );

    device_handle.claim_interface(inteface)?;

    device_handle.write_control(
        request_type,
        XMOS_DFU_RESETDEVICE,
        0,
        u16::from(inteface),
        &[],
        TIMEOUT,
    )?;

    device_handle.release_interface(inteface)?;

    info!("Reset was successfull.");

    Ok(())
}
