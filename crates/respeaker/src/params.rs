use std::{collections::HashMap, fmt::Display};

use clap::ValueEnum;
use eyre::Context;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[allow(clippy::upper_case_acronyms)] // ReSpeaker API uses UPPERCASE
#[allow(non_camel_case_types)] // ReSpeaker API uses UPPERCASE
#[derive(Clone, Debug, ValueEnum, EnumIter, Hash, PartialEq, Eq)]
#[clap(rename_all = "verbatim")]
pub enum ParamKind {
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

impl ParamKind {
    pub const fn def(&self) -> ParamDef {
        match self {
            Self::AECFREEZEONOFF => int_discrete(18, 7, Access::ReadWrite, "Adaptive Echo Canceler updates inhibit.", &[ "0 = Adaptation enabled", "1 = Freeze adaptation, filter only"]),
            Self::AECNORM => float_range(18, 19, 16., 0.25, Access::ReadWrite, "Limit on norm of AEC filter coefficients"),
            Self::AECPATHCHANGE => int_discrete(18, 25,  Access::ReadOnly, "AEC Path Change Detection.", &[ "0 = false (no path change detected)", "1 = true (path change detected)"]),
            Self::RT60 => float_range(18, 26, 0.9, 0.25, Access::ReadOnly, "Current RT60 estimate in seconds"),
            Self::HPFONOFF => int_discrete(18, 27, Access::ReadWrite, "High-pass Filter on microphone signals.", &["0 = OFF", "1 = ON - 70 Hz cut-off", "2 = ON - 125 Hz cut-off", "3 = ON - 180 Hz cut-off"]),
            Self::RT60ONOFF => int_discrete(18, 28,  Access::ReadWrite, "RT60 Estimation for AES.", &["0 = OFF", "1 = ON"]),
            Self::AECSILENCELEVEL => float_range(18, 30, 1., 1e-09, Access::ReadWrite, "Threshold for signal detection in AEC [-inf .. 0] dBov (Default: -80dBov = 10log10(1x10-8))"),
            Self::AECSILENCEMODE => int_discrete(18, 31,  Access::ReadOnly, "AEC far-end silence detection status. ", &["0 = false (signal detected) ", "1 = true (silence detected)"]),
            Self::AGCONOFF => int_discrete(19, 0,  Access::ReadWrite, "Automatic Gain Control. ", &[ "0 = OFF ", "1 = ON"]),
            Self::AGCMAXGAIN => float_range(19, 1, 1000., 1., Access::ReadWrite, "Maximum AGC gain factor. [0 .. 60] dB (default 30dB = 20log10(31.6))"),
            Self::AGCDESIREDLEVEL => float_range(19, 2, 0.99, 1e-08, Access::ReadWrite, "Target power level of the output signal. [-inf .. 0] dBov (default: -23dBov = 10log10(0.005))"),
            Self::AGCGAIN => float_range(19, 3, 1000., 1., Access::ReadWrite, "Current AGC gain factor. [0 .. 60] dB (default: 0.0dB = 20log10(1.0))"),
            Self::AGCTIME => float_range(19, 4, 1., 0.1, Access::ReadWrite, "Ramps-up / down time-constant in seconds."),
            Self::CNIONOFF => int_discrete(19, 5,  Access::ReadWrite, "Comfort Noise Insertion.", &["0 = OFF", "1 = ON"]),
            Self::FREEZEONOFF => int_discrete(19, 6,  Access::ReadWrite, "Adaptive beamformer updates.", &[ "0 = Adaptation enabled", "1 = Freeze adaptation, filter only"]),
            Self::STATNOISEONOFF => int_discrete(19, 8,  Access::ReadWrite, "Stationary noise suppression.", &[ "0 = OFF", "1 = ON"]),
            Self::GAMMA_NS => float_range(19, 9, 3., 0., Access::ReadWrite, "Over-subtraction factor of stationary noise. min .. max attenuation"),
            Self::MIN_NS => float_range(19, 10, 1., 0., Access::ReadWrite, "Gain-floor for stationary noise suppression. [-inf .. 0] dB (default: -16dB = 20log10(0.15))"),
            Self::NONSTATNOISEONOFF => int_discrete(19, 11,  Access::ReadWrite, "Non-stationary noise suppression.", &[ "0 = OFF", "1 = ON"]),
            Self::GAMMA_NN => float_range(19, 12, 3., 0., Access::ReadWrite, "Over-subtraction factor of non- stationary noise. min .. max attenuation"),
            Self::MIN_NN => float_range(19, 13, 1., 0., Access::ReadWrite, "Gain-floor for non-stationary noise suppression. [-inf .. 0] dB (default: -10dB = 20log10(0.3))"),
            Self::ECHOONOFF => int_discrete(19, 14,  Access::ReadWrite, "Echo suppression.", &[ "0 = OFF", "1 = ON"]),
            Self::GAMMA_E => float_range(19, 15, 3., 0., Access::ReadWrite, "Over-subtraction factor of echo (direct and early components). min .. max attenuation"),
            Self::GAMMA_ETAIL => float_range(19, 16, 3., 0., Access::ReadWrite, "Over-subtraction factor of echo (tail components). min .. max attenuation"),
            Self::GAMMA_ENL => float_range(19, 17, 5., 0., Access::ReadWrite, "Over-subtraction factor of non-linear echo. min .. max attenuation"),
            Self::NLATTENONOFF => int_discrete(19, 18, Access::ReadWrite, "Non-Linear echo attenuation.", &[ "0 = OFF", "1 = ON"]),
            Self::NLAEC_MODE => int_discrete(19, 20, Access::ReadWrite, "Non-Linear AEC training mode.", &[ "0 = OFF", "1 = ON - phase 1", "2 = ON - phase 2"]),
            Self::SPEECHDETECTED => int_discrete(19, 22, Access::ReadOnly, "Speech detection status.", &["0 = false (no speech detected)", "1 = true (speech detected)"]),
            Self::FSBUPDATED => int_discrete(19, 23, Access::ReadOnly, "FSB Update Decision.", &[ "0 = false (FSB was not updated)", "1 = true (FSB was updated)"]),
            Self::FSBPATHCHANGE => int_discrete(19, 24, Access::ReadOnly, "FSB Path Change Detection.", &["0 = false (no path change detected)", "1 = true (path change detected)"]),
            Self::TRANSIENTONOFF => int_discrete(19, 29, Access::ReadWrite, "Transient echo suppression.", &["0 = OFF", "1 = ON"]),
            Self::VOICEACTIVITY => int_discrete(19, 32, Access::ReadOnly, "VAD voice activity status.", &["0 = false (no voice activity)", "1 = true (voice activity)"]),
            Self::STATNOISEONOFF_SR => int_discrete(19, 33, Access::ReadWrite, "Stationary noise suppression for ASR.", &[ "0 = OFF", "1 = ON"]),
            Self::NONSTATNOISEONOFF_SR => int_discrete(19, 34, Access::ReadWrite, "Non-stationary noise suppression for ASR.", &["0 = OFF", "1 = ON"]),
            Self::GAMMA_NS_SR => float_range(19, 35, 3., 0., Access::ReadWrite, "Over-subtraction factor of stationary noise for ASR. [0.0 .. 3.0] (default: 1.0)"),
            Self::GAMMA_NN_SR => float_range(19, 36, 3., 0., Access::ReadWrite, "Over-subtraction factor of non-stationary noise for ASR. [0.0 .. 3.0] (default: 1.1)"),
            Self::MIN_NS_SR => float_range(19, 37, 1., 0., Access::ReadWrite, "Gain-floor for stationary noise suppression for ASR. [-inf .. 0] dB (default: -16dB = 20log10(0.15))"),
            Self::MIN_NN_SR => float_range(19, 38, 1., 0., Access::ReadWrite, "Gain-floor for non-stationary noise suppression for ASR. [-inf .. 0] dB (default: -10dB = 20log10(0.3))"),
            Self::GAMMAVAD_SR => float_range(19, 39, 1000., 0., Access::ReadWrite, "Set the threshold for voice activity detection. [-inf .. 60] dB (default: 3.5dB 20log10(1.5))"),
            Self::DOAANGLE => int_range(21, 0, 359, 0, Access::ReadOnly, "DOA angle. Current value. Orientation depends on build configuration.", &["[0 .. 359] Angle"])
        }
    }

    pub fn sorted() -> Vec<Self> {
        let mut params = Self::iter().collect::<Vec<_>>();
        params.sort_by_key(|p| {
            let def = p.def();
            (
                match def.access {
                    Access::ReadOnly => 1,
                    Access::ReadWrite => 0,
                },
                match def.param_type {
                    ParamType::IntDiscete { min: _, max: _ }
                    | ParamType::IntRange { min: _, max: _ } => 0,
                    ParamType::FloatRange { min: _, max: _ } => 1,
                },
            )
        });
        params
    }

    pub fn parse_value(&self, string: &str) -> eyre::Result<Value> {
        Ok(match self.def().param_type {
            ParamType::IntDiscete { min: _, max: _ } | ParamType::IntRange { min: _, max: _ } => {
                Value::Int(string.parse::<usize>().context("must be a usize")?)
            }
            ParamType::FloatRange { min: _, max: _ } => {
                Value::Float(string.parse::<f32>().context("must be an f32")?)
            }
        })
    }
}

#[derive(Debug)]
pub struct ParamDef {
    pub param_type: ParamType,
    pub id: u16,
    pub cmd: u16,
    pub access: Access,
    pub description: &'static str,
    pub value_descriptions: &'static [&'static str],
}

impl ParamDef {
    pub const fn min(&self) -> Value {
        match self.param_type {
            ParamType::IntDiscete { min, max: _ } | ParamType::IntRange { min, max: _ } => {
                Value::Int(min)
            }
            ParamType::FloatRange { min, max: _ } => Value::Float(min),
        }
    }

    pub const fn max(&self) -> Value {
        match self.param_type {
            ParamType::IntDiscete { min: _, max } | ParamType::IntRange { min: _, max } => {
                Value::Int(max)
            }
            ParamType::FloatRange { min: _, max } => Value::Float(max),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParamType {
    IntDiscete { min: usize, max: usize },
    IntRange { min: usize, max: usize },
    FloatRange { min: f32, max: f32 },
}

impl ParamType {
    pub const fn is_int(&self) -> bool {
        !matches!(self, Self::FloatRange { min: _, max: _ })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    ReadOnly,
    ReadWrite,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(usize),
    Float(f32),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(v) => f.write_str(&format!("{v}")),
            Self::Float(v) => f.write_str(&format!("{v}")),
        }
    }
}

const fn int_discrete<const N: usize>(
    id: u16,
    cmd: u16,
    access: Access,
    description: &'static str,
    value_descriptions: &'static [&'static str; N],
) -> ParamDef {
    ParamDef {
        param_type: ParamType::IntDiscete { min: 0, max: N - 1 },
        id,
        cmd,
        access,
        description,
        value_descriptions,
    }
}

const fn int_range(
    id: u16,
    cmd: u16,
    max: usize,
    min: usize,
    access: Access,
    description: &'static str,
    value_descriptions: &'static [&'static str; 1],
) -> ParamDef {
    ParamDef {
        param_type: ParamType::IntRange { min, max },
        id,
        cmd,
        access,
        description,
        value_descriptions,
    }
}

const fn float_range(
    id: u16,
    cmd: u16,
    max: f32,
    min: f32,
    access: Access,
    description: &'static str,
) -> ParamDef {
    ParamDef {
        param_type: ParamType::FloatRange { min, max },
        id,
        cmd,
        access,
        description,
        value_descriptions: &[],
    }
}

pub struct ParamState {
    pub current_params: HashMap<ParamKind, Value>,
}
