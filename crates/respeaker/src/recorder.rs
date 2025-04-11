use std::{
    collections::HashMap,
    fs::create_dir,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use eyre::Ok;
use tracing::info;

use crate::{
    csv::write_csv,
    params::{ParamKind, Value},
    respeaker_device::ReSpeakerDevice,
};

pub fn record_respeaker_parameters(
    seconds_to_record: f32,
    csv_path: Option<PathBuf>,
    device: &ReSpeakerDevice,
) -> eyre::Result<()> {
    let mut csv_data: Vec<(f32, HashMap<ParamKind, Value>)> = vec![];
    let start = Instant::now();
    while start.elapsed().as_secs_f32() > seconds_to_record {
        device.read_ro()?; // update readonly values
        let params = {
            let params = device
                .params()
                .lock()
                .expect("Lock failed")
                .current_params
                .clone();
            params
        };

        csv_data.push((start.elapsed().as_secs_f32(), params));
        thread::sleep(Duration::from_millis(10));
    }

    let dir = PathBuf::from("./recordings");
    if csv_path.is_none() && !dir.exists() {
        create_dir(dir)?;
    }
    let creation_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    let csv_path =
        csv_path.unwrap_or_else(|| PathBuf::from(format!("./recordings/{creation_time}.csv")));

    write_csv(csv_data, &csv_path)?;

    info!("Recording successful");

    Ok(())
}

// pub fn record_audio(
//     seconds_to_record: f32,
//     wav_path: Option<PathBuf>,
//     index_override: Option<usize>,
//     device: ReSpeakerDevice,
// ) -> eyre::Result<()> {
//     // mics
//     let host = cpal::default_host();
//     let audio_input_devices: Vec<_> = host.input_devices()?.collect();
//     info!(
//         "There are {} available audio input devices.",
//         audio_input_devices.len()
//     );
//     let mut respeaker_devices = vec![];
//     for (i, device) in audio_input_devices.iter().enumerate() {
//         let name = device.name().unwrap_or_else(|_| "?".into());
//         info!("Input device {i}: {name}");

//         if let Some(index) = index_override {
//             if i == index {
//                 respeaker_devices.push((i, device));
//                 break;
//             }
//         }

//         if name.starts_with("ReSpeaker 4 Mic Array") {
//             respeaker_devices.push((i, device));
//         }
//     }
//     if respeaker_devices.len() != 1 {
//         bail!("Device is ambiguous. Please provice an index with -m");
//     }

//     let (i, mic) = respeaker_devices.first().ok_or_eyre("Device not found")?;
//     info!("Using input device {i}");

//     let config_range = mic.default_input_config()?;
//     info!("Input device config: {:?}", &config_range);

//     let sample_format = config_range.sample_format();
//     let config: StreamConfig = config_range.into();

//     let (tx, rx) = mpsc::channel();

//     let (number_of_samples_to_record, spec, stream) = match sample_format {
//         cpal::SampleFormat::I16 => build_input_stream_i16(seconds_to_record, mic, &config, tx),
//         cpal::SampleFormat::F32 => build_input_stream_f32(seconds_to_record, mic, &config, tx),
//         _ => bail!("Only supporting I16 or 32 sample format for now"),
//     }?;

//     let dir = PathBuf::from("./recordings");
//     if wav_path.is_none() && !dir.exists() {
//         create_dir(dir)?;
//     }
//     let creation_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

//     let wav_path =
//         wav_path.unwrap_or_else(|| PathBuf::from(format!("./recordings/{creation_time}.wav")));
//     let mut writer = hound::WavWriter::create(&wav_path, spec)?;
//     let mut number_of_samples_written = 0;

//     info!(
//         "Recoring {} s of audio to file {:?}",
//         seconds_to_record, wav_path
//     );
//     stream.play()?;

//     let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

//     let join_handle: thread::JoinHandle<eyre::Result<Vec<(f32, HashMap<ParamKind, Value>)>>> =
//         thread::spawn(move || {
//             let mut csv_data: Vec<(f32, HashMap<ParamKind, Value>)> = vec![];
//             let start = Instant::now();
//             loop {
//                 if shutdown_rx.try_recv().is_ok() {
//                     info!("Refresh thread is shutting down");
//                     break;
//                 }

//                 device.read_ro()?; // update readonly values
//                 let params = {
//                     let params = device
//                         .params()
//                         .lock()
//                         .expect("Lock failed")
//                         .current_params
//                         .clone();
//                     params
//                 };

//                 csv_data.push((start.elapsed().as_secs_f32(), params));
//                 thread::sleep(Duration::from_millis(10));
//             }
//             eyre::Ok(csv_data)
//         });

//     loop {
//         if let Ok(chunk) = rx.recv() {
//             let len = chunk.samples.len();
//             match chunk.samples {
//                 Samples::F32(samples) => {
//                     for s in samples {
//                         writer.write_sample(s)?;
//                     }
//                 }
//                 Samples::I16(samples) => {
//                     for s in samples {
//                         writer.write_sample(s)?;
//                     }
//                 }
//             }

//             number_of_samples_written += len;

//             if number_of_samples_written >= number_of_samples_to_record {
//                 break;
//             }
//         }
//     }

//     drop(stream);

//     shutdown_tx.send(())?;
//     let csv_data = join_handle.join().unwrap()?;
//     let mut csv_path = wav_path.clone();
//     csv_path.set_extension("csv");
//     write_csv(csv_data, &csv_path)?;

//     info!("Recording successful");

//     Ok(())
// }

// #[derive(Debug)]
// pub struct AudioChunk {
//     // pub channels: u16,
//     // pub bits_per_sample: u16,
//     // pub sample_rate: u32,
//     pub samples: Samples,
// }

// #[derive(Debug)]
// pub enum Samples {
//     F32(Vec<f32>),
//     I16(Vec<i16>),
// }

// impl Samples {
//     fn len(&self) -> usize {
//         match self {
//             Self::F32(items) => items.len(),
//             Self::I16(items) => items.len(),
//         }
//     }
// }

// fn build_input_stream_f32(
//     seconds_to_record: f32,
//     device: &cpal::Device,
//     config: &cpal::StreamConfig,
//     tx: Sender<AudioChunk>,
// ) -> eyre::Result<(usize, hound::WavSpec, cpal::Stream)> {
//     let sample_rate = config.sample_rate.0;
//     let channels = config.channels;
//     let number_of_samples_to_record =
//         (sample_rate as f32 * seconds_to_record * channels as f32) as usize;

//     let err_fn = |err| error!("Stream error: {err}");

//     let spec = hound::WavSpec {
//         channels,
//         sample_rate,
//         bits_per_sample: 32,
//         sample_format: hound::SampleFormat::Float,
//     };

//     let stream = device.build_input_stream(
//         config,
//         move |data: &[f32], _: &cpal::InputCallbackInfo| {
//             let chunk = AudioChunk {
//                 // channels,
//                 // bits_per_sample: 32,
//                 // sample_rate,
//                 samples: Samples::F32(data.to_vec()),
//             };
//             trace!("Got audio chunk of length {}", chunk.samples.len());
//             tx.send(chunk).unwrap();
//         },
//         err_fn,
//         None,
//     )?;
//     Ok((number_of_samples_to_record, spec, stream))
// }

// fn build_input_stream_i16(
//     seconds_to_record: f32,
//     device: &cpal::Device,
//     config: &cpal::StreamConfig,
//     tx: Sender<AudioChunk>,
// ) -> eyre::Result<(usize, hound::WavSpec, cpal::Stream)> {
//     let sample_rate = config.sample_rate.0;
//     let channels = config.channels;
//     let number_of_samples_to_record =
//         (sample_rate as f32 * seconds_to_record * channels as f32) as usize;

//     let err_fn = |err| error!("Stream error: {err}");

//     let spec = hound::WavSpec {
//         channels,
//         sample_rate,
//         bits_per_sample: 16,
//         sample_format: hound::SampleFormat::Int,
//     };

//     let stream = device.build_input_stream(
//         config,
//         move |data: &[i16], _: &cpal::InputCallbackInfo| {
//             let chunk = AudioChunk {
//                 // channels,
//                 // bits_per_sample: 16,
//                 // sample_rate,
//                 samples: Samples::I16(data.to_vec()),
//             };
//             trace!("Got audio chunk of length {}", chunk.samples.len());
//             tx.send(chunk).unwrap();
//         },
//         err_fn,
//         None,
//     )?;
//     Ok((number_of_samples_to_record, spec, stream))
// }
