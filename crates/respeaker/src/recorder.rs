use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};

use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use eyre::{OptionExt, bail};
use tracing::{error, info, trace};

use crate::{
    csv::write_csv, params::{Param, Value}, respeaker_device::ReSpeakerDevice
};

pub fn record_audio(
    seconds_to_record: f32,
    wav_path: Option<PathBuf>,
    index_override: Option<usize>,
    device: ReSpeakerDevice,
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
        cpal::SampleFormat::F32 => build_input_stream(seconds_to_record, mic, &config, tx),
        _ => bail!("Only supporting F32 sample format for now"),
    }?;

    let wav_path = wav_path.unwrap_or_else(|| PathBuf::from("recording.wav"));
    let mut writer = hound::WavWriter::create(&wav_path, spec)?;
    let mut number_of_samples_written = 0;

    info!(
        "Recoring {} s of audio to file {:?}",
        seconds_to_record, wav_path
    );
    stream.play()?;

    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let join_handle: thread::JoinHandle<eyre::Result<Vec<(f32, HashMap<Param, Value>)>>> =
        thread::spawn(move || {
            let mut csv_data: Vec<(f32, HashMap<Param, Value>)> = vec![];
            let start = Instant::now();
            loop {
                if shutdown_rx.try_recv().is_ok() {
                    info!("Refresh thread is shutting down");
                    break;
                }

                device.read_ro()?; // update readonly values
                let params = {
                    let params = device.params().lock().unwrap().current_params.clone();
                    params
                };

                csv_data.push((start.elapsed().as_secs_f32(), params));
                thread::sleep(Duration::from_millis(10));
            }
            eyre::Ok(csv_data)
        });

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
    
    drop(stream);

    shutdown_tx.send(())?;
    let csv_data = join_handle.join().unwrap()?;
    write_csv(csv_data, &PathBuf::from("recording.csv") )?;

    info!("Recording successful");

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
