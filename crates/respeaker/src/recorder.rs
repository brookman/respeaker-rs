use std::{
    path::PathBuf,
    sync::mpsc::{self, Sender},
};

use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use eyre::{OptionExt, bail};
use tracing::{error, info, trace};

pub fn record_audio(
    seconds_to_record: f32,
    wav_path: Option<PathBuf>,
    index_override: Option<usize>,
) -> eyre::Result<()> {
    // mics
    let host = cpal::default_host();
    let audio_input_devices: Vec<_> = host.input_devices()?.collect();
    info!(
        "There are {} available audio input devices.",
        audio_input_devices.len()
    );
    let mut respeaker_devices = vec![];
    for (i, device) in audio_input_devices.iter().enumerate() {
        let name = device.name().unwrap_or_else(|_| "?".into());
        info!("Input device {i}: {name}");

        if let Some(index) = index_override {
            if i == index {
                respeaker_devices.push((i, device));
                break;
            }
        }

        if name.starts_with("ReSpeaker 4 Mic Array") {
            respeaker_devices.push((i, device));
        }
    }
    if respeaker_devices.len() != 1 {
        bail!("Device is ambiguous. Please provice an index with -m");
    }

    let (i, mic) = respeaker_devices.first().ok_or_eyre("Device not found")?;
    info!("Using input device {i}");

    let config_range = mic.default_input_config()?;
    info!("Input device config: {:?}", &config_range);

    let sample_format = config_range.sample_format();
    let config: StreamConfig = config_range.into();

    let (tx, rx) = mpsc::channel();

    let (number_of_samples_to_record, spec, stream) = match sample_format {
        cpal::SampleFormat::I8 => todo!(),
        cpal::SampleFormat::I16 => todo!(),
        cpal::SampleFormat::I32 => todo!(),
        cpal::SampleFormat::I64 => todo!(),
        cpal::SampleFormat::U8 => todo!(),
        cpal::SampleFormat::U16 => todo!(),
        cpal::SampleFormat::U32 => todo!(),
        cpal::SampleFormat::U64 => todo!(),
        cpal::SampleFormat::F32 => build_input_stream(seconds_to_record, mic, &config, tx),
        cpal::SampleFormat::F64 => todo!(),
        _ => todo!(),
    }?;

    let wav_path = wav_path.unwrap_or_else(|| PathBuf::from("recording.wav"));
    let mut writer = hound::WavWriter::create(&wav_path, spec)?;
    let mut number_of_samples_written = 0;

    info!(
        "Recoring {} s of audio to file {:?}",
        seconds_to_record, wav_path
    );
    stream.play()?;

    loop {
        if let Ok(chunk) = rx.recv() {
            let len = chunk.samples.len();
            for s in chunk.samples {
                writer.write_sample(s)?;
            }
            number_of_samples_written += len;

            if number_of_samples_written >= number_of_samples_to_record {
                break;
            }
        }
    }
    info!("Recording successful");
    drop(stream);

    Ok(())
}

#[derive(Debug)]
pub struct AudioChunk {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

fn build_input_stream(
    seconds_to_record: f32,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: Sender<AudioChunk>,
) -> eyre::Result<(usize, hound::WavSpec, cpal::Stream)> {
    let sample_rate = config.sample_rate.0;
    let channels = config.channels;
    let number_of_samples_to_record =
        (sample_rate as f32 * seconds_to_record * channels as f32) as usize;

    let err_fn = |err| error!("Stream error: {err}");

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let stream = device.build_input_stream(
        config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let chunk = AudioChunk {
                channels,
                sample_rate,
                samples: data.to_vec(),
            };
            trace!("Got audio chunk of length {}", chunk.samples.len());
            tx.send(chunk).unwrap();
        },
        err_fn,
        None,
    )?;
    Ok((number_of_samples_to_record, spec, stream))
}
