use std::{
    thread,
    time::{Duration, Instant},
};

use rusb::{Device, DeviceHandle, GlobalContext};
use strum::IntoEnumIterator;
use tabled::{Table, Tabled};
use tracing::info;

use crate::params::{Access, Param, ParamConfig, Value};
use eyre::{Result, bail};

const TIMEOUT: Duration = Duration::from_secs(2);

pub struct ReSpeakerDevice {
    index: usize,
    handle: DeviceHandle<GlobalContext>,
    interface_number: u8,
}

impl ReSpeakerDevice {
    pub fn open(device_index: Option<usize>) -> Result<Self> {
        fn open_internal(index: usize, device: &Device<GlobalContext>) -> Result<ReSpeakerDevice> {
            let handle = device.open()?;

            let config_desc = device.active_config_descriptor()?;
            for interface in config_desc.interfaces() {
                for interface_desc in interface.descriptors() {
                    if interface_desc.class_code() == 0xfe
                        && interface_desc.sub_class_code() == 0x01
                    {
                        let interface_number = interface_desc.interface_number();
                        return Ok(ReSpeakerDevice {
                            index,
                            handle,
                            interface_number,
                        });
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
                return open_internal(i, d);
            }
            bail!(
                "Device index (-i argument) out of range. Index was {i} but {} devices found.",
                devices.len()
            );
        }
        if devices.len() == 1 {
            return open_internal(0, &devices[0]);
        }
        if devices.len() > 1 {
            bail!("Multiple devices found. Specify the a device index with -i.")
        }

        bail!("No devices found")
    }

    pub fn read(&self, param_config: &ParamConfig) -> Result<Value> {
        let start = Instant::now();
        let (is_int, id, cmd) = match param_config {
            ParamConfig::IntMany(config) | ParamConfig::IntFew(config) => {
                (true, config.id, config.cmd)
            }
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

        self.handle
            .read_control(request_type, 0, cmd, id, &mut buffer, TIMEOUT)?;
        let response = (
            i32::from_le_bytes(buffer[0..4].try_into()?),
            i32::from_le_bytes(buffer[4..8].try_into()?),
        );
        info!("Read parameter in {:?}", start.elapsed());

        if is_int {
            if let ParamConfig::IntMany(config) | ParamConfig::IntFew(config) = param_config {
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

    pub fn write(&self, param: &Param, value: &Value) -> Result<()> {
        let config = param.config();

        let (id, cmd, access) = match config {
            ParamConfig::IntMany(config) | ParamConfig::IntFew(config) => {
                (config.id, config.cmd, config.access)
            }
            ParamConfig::Float(config) => (config.id, config.cmd, config.access),
        };

        if access == Access::ReadOnly {
            bail!("Parameter {:?} is read-only", param);
        }

        let (cmd_bytes, value_bytes, type_bytes) = match config {
            ParamConfig::IntMany(config) | ParamConfig::IntFew(config) => {
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

        self.handle
            .write_control(request_type, 0, 0, id, &payload, TIMEOUT)?;

        info!("Wrote value {value} to param {:?} successfully", param);

        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        const XMOS_DFU_RESETDEVICE: u8 = 0xF0;
        //const XMOS_DFU_REVERTFACTORY: u8 = 0xf1;

        let request_type = rusb::request_type(
            rusb::Direction::Out,
            rusb::RequestType::Class,
            rusb::Recipient::Interface,
        );

        self.handle.claim_interface(self.interface_number)?;

        self.handle.write_control(
            request_type,
            XMOS_DFU_RESETDEVICE,
            0,
            u16::from(self.interface_number),
            &[],
            TIMEOUT,
        )?;

        self.handle.release_interface(self.interface_number)?;

        info!("Reset was successfull.");
        thread::sleep(Duration::from_secs(2));

        *self = Self::open(Some(self.index))?;

        Ok(())
    }

    pub fn list(&self) -> Result<String> {
        let mut rows = vec![];
        for p in Param::iter() {
            let config = p.config();
            let value = self.read(config)?;
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
