use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use rusb::{Device, DeviceHandle, GlobalContext};
use strum::IntoEnumIterator;
use tabled::{Table, Tabled};
use tracing::info;

use crate::params::{Access, ParamKind, ParamState, ParamType, Value};
use eyre::{bail, OptionExt, Result};

const TIMEOUT: Duration = Duration::from_secs(2);

pub struct ReSpeakerDevice {
    index: usize,
    handle: DeviceHandle<GlobalContext>,
    interface_number: u8,
    param_state: Arc<Mutex<ParamState>>,
}

impl ReSpeakerDevice {
    pub fn open(device_index: Option<usize>, param_state: Arc<Mutex<ParamState>>) -> Result<Self> {
        fn open_internal(
            index: usize,
            device: &Device<GlobalContext>,
            param_state: Arc<Mutex<ParamState>>,
        ) -> Result<ReSpeakerDevice> {
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
                            param_state,
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
                return open_internal(i, d, param_state);
            }
            bail!(
                "Device index (-i argument) out of range. Index was {i} but {} devices found.",
                devices.len()
            );
        }
        if devices.len() == 1 {
            return open_internal(0, &devices[0], param_state);
        }
        if devices.len() > 1 {
            bail!("Multiple devices found. Specify the a device index with -i.")
        }

        bail!("No devices found")
    }

    pub fn read(&self, param: &ParamKind) -> Result<Value> {
        let value = self.read_internal(param)?;
        {
            let mut params = self.param_state.lock().expect("Lock failed");
            params.current_params.insert(param.clone(), value.clone());
        }
        Ok(value)
    }

    fn read_internal(&self, param: &ParamKind) -> Result<Value> {
        let start = Instant::now();
        let def = param.def();

        let mut cmd = 0x80 | def.cmd;
        if def.param_type.is_int() {
            cmd |= 0x40;
        }

        let mut buffer = [0u8; 8];

        let request_type = rusb::request_type(
            rusb::Direction::In,
            rusb::RequestType::Vendor,
            rusb::Recipient::Device,
        );

        self.handle
            .read_control(request_type, 0, cmd, def.id, &mut buffer, TIMEOUT)?;
        let response = (
            i32::from_le_bytes(buffer[0..4].try_into()?),
            i32::from_le_bytes(buffer[4..8].try_into()?),
        );
        info!("Read parameter {:?} in {:?}", param, start.elapsed());

        Ok(if def.param_type.is_int() {
            Value::Int(response.0 as usize)
        } else {
            #[allow(clippy::cast_possible_truncation)]
            let float = (f64::from(response.0) * f64::from(response.1).exp2()) as f32;
            Value::Float(float)
        })
    }

    fn read_all(&self) -> Result<HashMap<ParamKind, Value>> {
        let mut result = HashMap::new();

        for p in ParamKind::iter() {
            let value = self.read(&p)?;
            result.insert(p, value);
        }

        Ok(result)
    }

    pub fn read_ro(&self) -> Result<HashMap<ParamKind, Value>> {
        let mut result = HashMap::new();

        for p in ParamKind::iter().filter(|p| p.def().access == Access::ReadOnly) {
            let value = self.read(&p)?;
            result.insert(p, value);
        }

        Ok(result)
    }

    pub fn write(&self, param: &ParamKind, value: &Value) -> Result<()> {
        let def = param.def();

        if def.access == Access::ReadOnly {
            bail!("Parameter {:?} is read-only", param);
        }

        let (value_bytes, type_bytes) = match def.param_type {
            ParamType::IntDiscete { min, max } | ParamType::IntRange { min, max } => match value {
                Value::Int(value) => {
                    if value < &min || value > &max {
                        bail!("Value {value} is not in range {}..={}", min, max);
                    }
                    ((*value as i32).to_le_bytes(), 1i32.to_le_bytes())
                }
                Value::Float(_) => {
                    bail!("Parameter type and value mismatch. Value must be i32 but was f32");
                }
            },
            ParamType::FloatRange { min, max } => match value {
                Value::Int(_) => {
                    bail!("Parameter type and value mismatch. Value must be f32 but was i32");
                }
                Value::Float(value) => {
                    if value < &min || value > &max {
                        bail!("Value {value} is not in range {}..={}", min, max);
                    }
                    (value.to_le_bytes(), 0i32.to_le_bytes())
                }
            },
        };

        let cmd_bytes = i32::from(def.cmd).to_le_bytes();

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
            .write_control(request_type, 0, 0, def.id, &payload, TIMEOUT)?;

        info!("Wrote value {value} to param {:?} successfully", param);

        {
            let mut params = self.param_state.lock().expect("Lock failed");
            params.current_params.insert(param.clone(), value.clone());
        }

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

        *self = Self::open(Some(self.index), self.param_state.clone())?;

        Ok(())
    }

    pub fn list(&self) -> Result<String> {
        let param_map = self.read_all()?;
        let mut rows = vec![];
        for p in ParamKind::iter() {
            let def = p.def();

            let value = param_map.get(&p).ok_or_eyre("Param not found")?;

            let t = if def.param_type.is_int() {
                "int"
            } else {
                "float"
            };

            rows.push(TableRow {
                name: format!("{p:?}"),
                value: value.clone(),
                t: t.to_string(),
                access: if def.access == Access::ReadOnly {
                    "ro"
                } else {
                    "rw"
                }
                .to_string(),
                range: format!("{}..{}", def.min(), def.max()),
                description: def.description.to_string(),
                values: def.value_descriptions.join("\n"),
            });
        }
        Ok(Table::new(rows).to_string())
    }

    pub fn params(&self) -> Arc<Mutex<ParamState>> {
        self.param_state.clone()
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
