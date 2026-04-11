use std::{
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use cpal::DeviceId;
use cpal::traits::{DeviceTrait, StreamTrait};
use inazuma_util::ResultExt;

pub struct CaptureInput {
    pub name: String,
    pub input_device: Option<DeviceId>,
    config: cpal::SupportedStreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
    _stream: cpal::Stream,
}

impl CaptureInput {
    pub fn start(input_device: Option<DeviceId>) -> anyhow::Result<Self> {
        let (device, config) = crate::default_device(true, input_device.as_ref())?;
        let name = device
            .description()
            .map(|desc| desc.name().to_string())
            .unwrap_or("<unknown>".to_string());
        log::info!("Using microphone: {}", name);

        let samples = Arc::new(Mutex::new(Vec::new()));
        let stream = start_capture(device, config.clone(), samples.clone())?;

        Ok(Self {
            name,
            input_device,
            _stream: stream,
            config,
            samples,
        })
    }

    pub fn finish(self) -> Result<PathBuf> {
        let name = self.name;
        let mut path = env::current_dir().context("Could not get current dir")?;
        path.push(&format!("test_recording_{name}.wav"));
        log::info!("Test recording written to: {}", path.display());
        write_out(self.samples, self.config, &path)?;
        Ok(path)
    }
}

fn start_capture(
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    samples: Arc<Mutex<Vec<i16>>>,
) -> Result<cpal::Stream> {
    let stream = device
        .build_input_stream_raw(
            &config.config(),
            config.sample_format(),
            move |data, _: &_| {
                let data = crate::get_sample_data(config.sample_format(), data).log_err();
                let Some(data) = data else {
                    return;
                };
                samples
                    .try_lock()
                    .expect("Only locked after stream ends")
                    .extend_from_slice(&data);
            },
            |err| log::error!("error capturing audio track: {:?}", err),
            Some(Duration::from_millis(100)),
        )
        .context("failed to build input stream")?;

    stream.play()?;
    Ok(stream)
}

fn write_out(
    samples: Arc<Mutex<Vec<i16>>>,
    config: cpal::SupportedStreamConfig,
    path: &Path,
) -> Result<()> {
    let samples = std::mem::take(
        &mut *samples
            .try_lock()
            .expect("Stream has ended, callback cant hold the lock"),
    );
    let channels = config.channels();
    let sample_rate = config.sample_rate();
    write_wav(path, &samples, channels, sample_rate)
}

/// Writes raw i16 PCM samples to a WAV file.
fn write_wav(path: &Path, samples: &[i16], channels: u16, sample_rate: u32) -> Result<()> {
    use std::io::Write;

    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size = (samples.len() * 2) as u32;
    let file_size = 36 + data_size;

    let mut file = std::fs::File::create(path).context("failed to create WAV file")?;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt sub-chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // sub-chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM format
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data sub-chunk
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}
