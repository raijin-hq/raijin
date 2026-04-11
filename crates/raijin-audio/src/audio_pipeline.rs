use anyhow::{Context as _, Result};
use inazuma_collections::HashMap;
use cpal::{
    DeviceDescription, DeviceId, default_host,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use inazuma::{App, AsyncApp, BorrowAppContext, Global};

use rodio::{Decoder, Source, dynamic_mixer, source::Buffered, source::SamplesConverter};
use inazuma_settings_framework::Settings;
use std::io::Cursor;
use inazuma_util::ResultExt;

mod echo_canceller;
use echo_canceller::EchoCanceller;
mod rodio_ext;
pub use crate::audio_settings::AudioSettings;
pub use rodio_ext::RodioExt;

use crate::audio_settings::LIVE_SETTINGS;

use crate::Sound;

use super::{CHANNEL_COUNT, SAMPLE_RATE};
pub const BUFFER_SIZE: usize = // echo canceller and livekit want 10ms of audio
    (SAMPLE_RATE as usize / 100) * CHANNEL_COUNT as usize;

pub fn init(cx: &mut App) {
    LIVE_SETTINGS.initialize(cx);
}

// TODO(jk): this is currently cached only once - we should observe and react instead
pub fn ensure_devices_initialized(cx: &mut App) {
    if cx.has_global::<AvailableAudioDevices>() {
        return;
    }
    cx.default_global::<AvailableAudioDevices>();
    let task = cx
        .background_executor()
        .spawn(async move { get_available_audio_devices() });
    cx.spawn(async move |cx: &mut AsyncApp| {
        let devices = task.await;
        cx.update(|cx| cx.set_global(AvailableAudioDevices(devices)));
        cx.refresh();
    })
    .detach();
}

/// Holds the output stream and mixer controller for audio playback.
/// The cpal output stream reads from the mixer's output and writes to the device.
pub struct AudioOutput {
    /// Must be kept alive for audio to play. Dropping this stops all audio output.
    _output_stream: cpal::Stream,
    mixer_controller: std::sync::Arc<dynamic_mixer::DynamicMixerController<f32>>,
}

pub struct Audio {
    output: Option<AudioOutput>,
    pub echo_canceller: EchoCanceller,
    source_cache: HashMap<Sound, Buffered<SamplesConverter<Decoder<Cursor<Vec<u8>>>, f32>>>,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            output: None,
            echo_canceller: EchoCanceller::default(),
            source_cache: HashMap::default(),
        }
    }
}

impl Global for Audio {}

impl Audio {
    fn ensure_output_exists(
        &mut self,
        output_audio_device: Option<DeviceId>,
    ) -> Result<&std::sync::Arc<dynamic_mixer::DynamicMixerController<f32>>> {
        #[cfg(debug_assertions)]
        log::warn!(
            "Audio does not sound correct without optimizations. Use a release build to debug audio issues"
        );

        if self.output.is_none() {
            let audio_output =
                open_output_stream(output_audio_device, self.echo_canceller.clone())?;
            self.output = Some(audio_output);
        }

        Ok(self
            .output
            .as_ref()
            .map(|o| &o.mixer_controller)
            .expect("we only get here if opening the outputstream succeeded"))
    }

    pub fn play_sound(sound: Sound, cx: &mut App) {
        let output_audio_device = AudioSettings::get_global(cx).output_audio_device.clone();
        cx.update_default_global(|this: &mut Self, cx| {
            let source = this.sound_source(sound, cx).log_err()?;
            let output_mixer = this
                .ensure_output_exists(output_audio_device)
                .context("Could not get output mixer")
                .log_err()?;

            output_mixer.add(source);
            Some(())
        });
    }

    pub fn end_call(cx: &mut App) {
        cx.update_default_global(|this: &mut Self, _cx| {
            this.output.take();
        });
    }

    fn sound_source(&mut self, sound: Sound, cx: &App) -> Result<impl Source<Item = f32> + Send + 'static> {
        if let Some(wav) = self.source_cache.get(&sound) {
            return Ok(wav.clone());
        }

        let path = format!("sounds/{}.wav", sound.file());
        let bytes = cx
            .asset_source()
            .load(&path)?
            .map(anyhow::Ok)
            .with_context(|| format!("No asset available for path {path}"))??
            .into_owned();
        let cursor = Cursor::new(bytes);
        let source: SamplesConverter<Decoder<Cursor<Vec<u8>>>, f32> = Decoder::new(cursor)?.convert_samples();
        let source = source.buffered();

        self.source_cache.insert(sound, source.clone());

        Ok(source)
    }
}

/// An opened input (microphone) stream using cpal directly.
pub struct InputStream {
    _stream: cpal::Stream,
    config: cpal::StreamConfig,
}

impl InputStream {
    pub fn config(&self) -> &cpal::StreamConfig {
        &self.config
    }
}

impl std::fmt::Debug for InputStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputStream")
            .field("config", &self.config)
            .finish()
    }
}

pub fn open_input_stream(
    device_id: Option<DeviceId>,
) -> anyhow::Result<InputStream> {
    let device = resolve_device(device_id.as_ref(), true)?;
    let supported_config = device
        .default_input_config()
        .context("no default input config available")?;

    let config = cpal::StreamConfig {
        channels: supported_config.channels(),
        sample_rate: supported_config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = device
        .build_input_stream(
            &config,
            |_data: &[f32], _info: &cpal::InputCallbackInfo| {
                // Input data is processed by the echo canceller pipeline
            },
            |err| {
                log::error!("Input stream error: {}", err);
            },
            None,
        )
        .context("Could not build input stream")?;

    stream.play().context("Could not start input stream")?;

    log::info!("Opened microphone: {:?}", config);
    Ok(InputStream {
        _stream: stream,
        config,
    })
}

pub fn resolve_device(device_id: Option<&DeviceId>, input: bool) -> anyhow::Result<cpal::Device> {
    if let Some(id) = device_id {
        if let Some(device) = default_host().device_by_id(id) {
            return Ok(device);
        }
        log::warn!("Selected audio device not found, falling back to default");
    }
    if input {
        default_host()
            .default_input_device()
            .context("no audio input device available")
    } else {
        default_host()
            .default_output_device()
            .context("no audio output device available")
    }
}

/// Opens a test output stream on the given device. Returns a mixer controller
/// that can be used to add sources for playback.
pub fn open_test_output(
    device_id: Option<DeviceId>,
) -> anyhow::Result<(cpal::Stream, std::sync::Arc<dynamic_mixer::DynamicMixerController<f32>>)> {
    let device = resolve_device(device_id.as_ref(), false)?;
    let supported_config = device
        .default_output_config()
        .context("no default output config")?;

    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels();

    let (controller, mixer) = dynamic_mixer::mixer::<f32>(channels, sample_rate);

    let config = cpal::StreamConfig {
        channels,
        sample_rate: sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let mut source = Box::new(mixer) as Box<dyn Source<Item = f32> + Send>;
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    *sample = source.next().unwrap_or(0.0);
                }
            },
            |err| {
                log::error!("Output stream error: {}", err);
            },
            None,
        )
        .context("Could not build output stream")?;

    stream.play().context("Could not start output stream")?;

    Ok((stream, controller))
}

pub fn open_output_stream(
    device_id: Option<DeviceId>,
    mut echo_canceller: EchoCanceller,
) -> anyhow::Result<AudioOutput> {
    let device = resolve_device(device_id.as_ref(), false)?;
    let supported_config = device
        .default_output_config()
        .context("no default output config")?;

    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels();

    let (mixer_controller, mixer) = dynamic_mixer::mixer::<f32>(channels, sample_rate);
    // Keep the mixer alive by adding a zero source so it never ends
    mixer_controller.add(rodio::source::Zero::<f32>::new(channels, sample_rate));

    let echo_cancelling_source = mixer
        .inspect_buffer::<BUFFER_SIZE, _>(move |buffer| {
            let mut buf: [i16; BUFFER_SIZE] = std::array::from_fn(|i| {
                (buffer[i] * i16::MAX as f32) as i16
            });
            echo_canceller.process_reverse_stream(&mut buf)
        });

    let config = cpal::StreamConfig {
        channels,
        sample_rate: sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let mut source = Box::new(echo_cancelling_source) as Box<dyn Source<Item = f32> + Send>;
    let output_stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    *sample = source.next().unwrap_or(0.0);
                }
            },
            |err| {
                log::error!("Output stream error: {}", err);
            },
            None,
        )
        .context("Could not build output stream")?;

    output_stream.play().context("Could not start output stream")?;

    log::info!("Output stream opened");

    Ok(AudioOutput {
        _output_stream: output_stream,
        mixer_controller,
    })
}

#[derive(Clone, Debug)]
pub struct AudioDeviceInfo {
    pub id: DeviceId,
    pub desc: DeviceDescription,
}

impl AudioDeviceInfo {
    pub fn matches_input(&self, is_input: bool) -> bool {
        if is_input {
            self.desc.supports_input()
        } else {
            self.desc.supports_output()
        }
    }

    pub fn matches(&self, id: &DeviceId, is_input: bool) -> bool {
        &self.id == id && self.matches_input(is_input)
    }
}

impl std::fmt::Display for AudioDeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.desc.name(), self.id)
    }
}

fn get_available_audio_devices() -> Vec<AudioDeviceInfo> {
    let Some(devices) = default_host().devices().ok() else {
        return Vec::new();
    };
    devices
        .filter_map(|device| {
            let id = device.id().ok()?;
            let desc = device.description().ok()?;
            Some(AudioDeviceInfo { id, desc })
        })
        .collect()
}

#[derive(Default, Clone, Debug)]
pub struct AvailableAudioDevices(pub Vec<AudioDeviceInfo>);

impl Global for AvailableAudioDevices {}
