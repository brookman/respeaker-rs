use eyre::{Context, Result};
use std::{collections::HashMap, fmt::Display, path::Iter, sync::OnceLock};
use strum::IntoEnumIterator;

use clap::ValueEnum;
use enum_map::{Enum, EnumMap, enum_map};
use strum_macros::EnumIter;

#[allow(clippy::upper_case_acronyms)] // ReSpeaker API uses UPPERCASE
#[allow(non_camel_case_types)] // ReSpeaker API uses UPPERCASE
#[derive(Clone, Debug, Enum, ValueEnum, EnumIter, Hash, PartialEq, Eq)]
#[clap(rename_all = "verbatim")]
pub enum Param {
    AECFREEZEONOFF,
    AECNORM,
    AECPATHCHANGE,
    AECSILENCELEVEL,
    AECSILENCEMODE,
    AGCDESIREDLEVEL,
    AGCGAIN,
    AGCMAXGAIN,
    AGCONOFF,
    AGCTIME,
    CNIONOFF,
    DOAANGLE,
    ECHOONOFF,
    FREEZEONOFF,
    FSBPATHCHANGE,
    FSBUPDATED,
    GAMMAVAD_SR,
    GAMMA_E,
    GAMMA_ENL,
    GAMMA_ETAIL,
    GAMMA_NN,
    GAMMA_NN_SR,
    GAMMA_NS,
    GAMMA_NS_SR,
    HPFONOFF,
    MIN_NN,
    MIN_NN_SR,
    MIN_NS,
    MIN_NS_SR,
    NLAEC_MODE,
    NLATTENONOFF,
    NONSTATNOISEONOFF,
    NONSTATNOISEONOFF_SR,
    RT60,
    RT60ONOFF,
    SPEECHDETECTED,
    STATNOISEONOFF,
    STATNOISEONOFF_SR,
    TRANSIENTONOFF,
    VOICEACTIVITY,
}

impl Param {
    pub fn config(&self) -> &ParamConfig {
        static MAP: OnceLock<EnumMap<Param, ParamConfig>> = OnceLock::new();
        &MAP.get_or_init(|| {
            enum_map! {
                Self::AECFREEZEONOFF => ParamConfig::int2(18, 7, 1, 0, Access::ReadWrite, "Adaptive Echo Canceler updates inhibit.", "0 = Adaptation enabled", "1 = Freeze adaptation, filter only"),
                Self::AECNORM => ParamConfig::float(18, 19, 16., 0.25, Access::ReadWrite, "Limit on norm of AEC filter coefficients"),
                Self::AECPATHCHANGE => ParamConfig::int2(18, 25, 1, 0, Access::ReadOnly, "AEC Path Change Detection.", "0 = false (no path change detected)", "1 = true (path change detected)"),
                Self::RT60 => ParamConfig::float(18, 26, 0.9, 0.25, Access::ReadOnly, "Current RT60 estimate in seconds"),
                Self::HPFONOFF => ParamConfig::int4(18, 27, 3, 0, Access::ReadWrite, "High-pass Filter on microphone signals.", "0 = OFF", "1 = ON - 70 Hz cut-off", "2 = ON - 125 Hz cut-off", "3 = ON - 180 Hz cut-off"),
                Self::RT60ONOFF => ParamConfig::int2(18, 28, 1, 0, Access::ReadWrite, "RT60 Estimation for AES.", "0 = OFF", "1 = ON"),
                Self::AECSILENCELEVEL => ParamConfig::float(18, 30, 1., 1e-09, Access::ReadWrite, "Threshold for signal detection in AEC [-inf .. 0] dBov (Default: -80dBov = 10log10(1x10-8))"),
                Self::AECSILENCEMODE => ParamConfig::int2(18, 31, 1, 0, Access::ReadOnly, "AEC far-end silence detection status. ", "0 = false (signal detected) ", "1 = true (silence detected)"),
                Self::AGCONOFF => ParamConfig::int2(19, 0, 1, 0, Access::ReadWrite, "Automatic Gain Control. ", "0 = OFF ", "1 = ON"),
                Self::AGCMAXGAIN => ParamConfig::float(19, 1, 1000., 1., Access::ReadWrite, "Maximum AGC gain factor. [0 .. 60] dB (default 30dB = 20log10(31.6))"),
                Self::AGCDESIREDLEVEL => ParamConfig::float(19, 2, 0.99, 1e-08, Access::ReadWrite, "Target power level of the output signal. [-inf .. 0] dBov (default: -23dBov = 10log10(0.005))"),
                Self::AGCGAIN => ParamConfig::float(19, 3, 1000., 1., Access::ReadWrite, "Current AGC gain factor. [0 .. 60] dB (default: 0.0dB = 20log10(1.0))"),
                Self::AGCTIME => ParamConfig::float(19, 4, 1., 0.1, Access::ReadWrite, "Ramps-up / down time-constant in seconds."),
                Self::CNIONOFF => ParamConfig::int2(19, 5, 1, 0, Access::ReadWrite, "Comfort Noise Insertion.", "0 = OFF", "1 = ON"),
                Self::FREEZEONOFF => ParamConfig::int2(19, 6, 1, 0, Access::ReadWrite, "Adaptive beamformer updates.", "0 = Adaptation enabled", "1 = Freeze adaptation, filter only"),
                Self::STATNOISEONOFF => ParamConfig::int2(19, 8, 1, 0, Access::ReadWrite, "Stationary noise suppression.", "0 = OFF", "1 = ON"),
                Self::GAMMA_NS => ParamConfig::float(19, 9, 3., 0., Access::ReadWrite, "Over-subtraction factor of stationary noise. min .. max attenuation"),
                Self::MIN_NS => ParamConfig::float(19, 10, 1., 0., Access::ReadWrite, "Gain-floor for stationary noise suppression. [-inf .. 0] dB (default: -16dB = 20log10(0.15))"),
                Self::NONSTATNOISEONOFF => ParamConfig::int2(19, 11, 1, 0, Access::ReadWrite, "Non-stationary noise suppression.", "0 = OFF", "1 = ON"),
                Self::GAMMA_NN => ParamConfig::float(19, 12, 3., 0., Access::ReadWrite, "Over-subtraction factor of non- stationary noise. min .. max attenuation"),
                Self::MIN_NN => ParamConfig::float(19, 13, 1., 0., Access::ReadWrite, "Gain-floor for non-stationary noise suppression. [-inf .. 0] dB (default: -10dB = 20log10(0.3))"),
                Self::ECHOONOFF => ParamConfig::int2(19, 14, 1, 0, Access::ReadWrite, "Echo suppression.", "0 = OFF", "1 = ON"),
                Self::GAMMA_E => ParamConfig::float(19, 15, 3., 0., Access::ReadWrite, "Over-subtraction factor of echo (direct and early components). min .. max attenuation"),
                Self::GAMMA_ETAIL => ParamConfig::float(19, 16, 3., 0., Access::ReadWrite, "Over-subtraction factor of echo (tail components). min .. max attenuation"),
                Self::GAMMA_ENL => ParamConfig::float(19, 17, 5., 0., Access::ReadWrite, "Over-subtraction factor of non-linear echo. min .. max attenuation"),
                Self::NLATTENONOFF => ParamConfig::int2(19, 18, 1, 0, Access::ReadWrite, "Non-Linear echo attenuation.", "0 = OFF", "1 = ON"),
                Self::NLAEC_MODE => ParamConfig::int3(19, 20, 2, 0, Access::ReadWrite, "Non-Linear AEC training mode.", "0 = OFF", "1 = ON - phase 1", "2 = ON - phase 2"),
                Self::SPEECHDETECTED => ParamConfig::int2(19, 22, 1, 0, Access::ReadOnly, "Speech detection status.", "0 = false (no speech detected)", "1 = true (speech detected)"),
                Self::FSBUPDATED => ParamConfig::int2(19, 23, 1, 0, Access::ReadOnly, "FSB Update Decision.", "0 = false (FSB was not updated)", "1 = true (FSB was updated)"),
                Self::FSBPATHCHANGE => ParamConfig::int2(19, 24, 1, 0, Access::ReadOnly, "FSB Path Change Detection.", "0 = false (no path change detected)", "1 = true (path change detected)"),
                Self::TRANSIENTONOFF => ParamConfig::int2(19, 29, 1, 0, Access::ReadWrite, "Transient echo suppression.", "0 = OFF", "1 = ON"),
                Self::VOICEACTIVITY => ParamConfig::int2(19, 32, 1, 0, Access::ReadOnly, "VAD voice activity status.", "0 = false (no voice activity)", "1 = true (voice activity)"),
                Self::STATNOISEONOFF_SR => ParamConfig::int2(19, 33, 1, 0, Access::ReadWrite, "Stationary noise suppression for ASR.", "0 = OFF", "1 = ON"),
                Self::NONSTATNOISEONOFF_SR => ParamConfig::int2(19, 34, 1, 0, Access::ReadWrite, "Non-stationary noise suppression for ASR.", "0 = OFF", "1 = ON"),
                Self::GAMMA_NS_SR => ParamConfig::float(19, 35, 3., 0., Access::ReadWrite, "Over-subtraction factor of stationary noise for ASR. [0.0 .. 3.0] (default: 1.0)"),
                Self::GAMMA_NN_SR => ParamConfig::float(19, 36, 3., 0., Access::ReadWrite, "Over-subtraction factor of non-stationary noise for ASR. [0.0 .. 3.0] (default: 1.1)"),
                Self::MIN_NS_SR => ParamConfig::float(19, 37, 1., 0., Access::ReadWrite, "Gain-floor for stationary noise suppression for ASR. [-inf .. 0] dB (default: -16dB = 20log10(0.15))"),
                Self::MIN_NN_SR => ParamConfig::float(19, 38, 1., 0., Access::ReadWrite, "Gain-floor for non-stationary noise suppression for ASR. [-inf .. 0] dB (default: -10dB = 20log10(0.3))"),
                Self::GAMMAVAD_SR => ParamConfig::float(19, 39, 1000., 0., Access::ReadWrite, "Set the threshold for voice activity detection. [-inf .. 60] dB (default: 3.5dB 20log10(1.5))"),
                Self::DOAANGLE => ParamConfig::int_n(21, 0, 359, 0, Access::ReadOnly, "DOA angle. Current value. Orientation depends on build configuration.", "[0 .. 359] Angle")
            }
        })[self.clone()]
    }

    pub fn sorted() -> Vec<Self> {
        let mut params = Self::iter().collect::<Vec<_>>();
        params.sort_by_key(|p| {
            let config = p.config();
            (
                match config.access() {
                    Access::ReadOnly => 1,
                    Access::ReadWrite => 0,
                },
                match config {
                    ParamConfig::IntMany(_) | ParamConfig::IntFew(_) => 0,
                    ParamConfig::Float(_) => 1,
                },
            )
        });
        params
    }
}

#[derive(Debug, Clone)]
pub enum ParamConfig {
    IntMany(Config<i32>),
    IntFew(Config<i32>),
    Float(Config<f32>),
}

impl ParamConfig {
    fn int_n(
        id: u16,
        cmd: u16,
        max: i32,
        min: i32,
        access: Access,
        description: &str,
        value_description_1: &str,
    ) -> Self {
        Self::IntMany(Config {
            id,
            cmd,
            min,
            max,
            access,
            description: description.to_string(),
            value_descriptions: vec![value_description_1.to_string()],
        })
    }

    fn int2(
        id: u16,
        cmd: u16,
        max: i32,
        min: i32,
        access: Access,
        description: &str,
        value_description_1: &str,
        value_description_2: &str,
    ) -> Self {
        Self::IntFew(Config {
            id,
            cmd,
            min,
            max,
            access,
            description: description.to_string(),
            value_descriptions: vec![
                value_description_1.to_string(),
                value_description_2.to_string(),
            ],
        })
    }

    fn int3(
        id: u16,
        cmd: u16,
        max: i32,
        min: i32,
        access: Access,
        description: &str,
        value_description_1: &str,
        value_description_2: &str,
        value_description_3: &str,
    ) -> Self {
        Self::IntFew(Config {
            id,
            cmd,
            min,
            max,
            access,
            description: description.to_string(),
            value_descriptions: vec![
                value_description_1.to_string(),
                value_description_2.to_string(),
                value_description_3.to_string(),
            ],
        })
    }

    fn int4(
        id: u16,
        cmd: u16,
        max: i32,
        min: i32,
        access: Access,
        description: &str,
        value_description_1: &str,
        value_description_2: &str,
        value_description_3: &str,
        value_description_4: &str,
    ) -> Self {
        Self::IntFew(Config {
            id,
            cmd,
            min,
            max,
            access,
            description: description.to_string(),
            value_descriptions: vec![
                value_description_1.to_string(),
                value_description_2.to_string(),
                value_description_3.to_string(),
                value_description_4.to_string(),
            ],
        })
    }

    fn float(id: u16, cmd: u16, max: f32, min: f32, access: Access, description: &str) -> Self {
        Self::Float(Config {
            id,
            cmd,
            min,
            max,
            access,
            description: description.to_string(),
            value_descriptions: vec![],
        })
    }

    pub fn access(&self) -> Access {
        match self {
            ParamConfig::IntMany(config) => config.access,
            ParamConfig::IntFew(config) => config.access,
            ParamConfig::Float(config) => config.access,
        }
    }

    pub fn description(&self) -> String {
        match self {
            ParamConfig::IntMany(config) => config.description.clone(),
            ParamConfig::IntFew(config) => config.description.clone(),
            ParamConfig::Float(config) => config.description.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config<T> {
    pub id: u16,
    pub cmd: u16,
    pub min: T,
    pub max: T,
    pub access: Access,
    pub description: String,
    pub value_descriptions: Vec<String>,
}

pub trait ParseValue {
    fn parse_value(&self, string: &str) -> Result<Value>;
}

impl ParseValue for Config<i32> {
    fn parse_value(&self, string: &str) -> Result<Value> {
        Ok(Value::Int(
            self.clone(),
            string.parse::<i32>().context("must be an i32")?,
        ))
    }
}

impl ParseValue for Config<f32> {
    fn parse_value(&self, string: &str) -> Result<Value> {
        Ok(Value::Float(
            self.clone(),
            string.parse::<f32>().context("must be an f32")?,
        ))
    }
}

impl ParseValue for ParamConfig {
    fn parse_value(&self, string: &str) -> Result<Value> {
        match self {
            Self::IntFew(config) | Self::IntMany(config) => config.parse_value(string),
            Self::Float(config) => config.parse_value(string),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(Config<i32>, i32),
    Float(Config<f32>, f32),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(_, v) => f.write_str(&format!("{v}")),
            Self::Float(_, v) => f.write_str(&format!("{v}")),
        }
    }
}

pub struct ParamState {
    pub current_params: HashMap<Param, Value>,
}
