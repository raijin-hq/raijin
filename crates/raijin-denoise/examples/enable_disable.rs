use std::time::Duration;

use rodio::Source;
use rodio::wav_to_file;
use rodio::source::UniformSourceIterator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::open("clips_airconditioning.wav")?;
    let decoder = rodio::Decoder::try_from(file)?;
    let resampled = UniformSourceIterator::new(decoder, 1, 16_000);

    let mut enabled = true;
    let denoised = raijin_denoise::Denoiser::try_new(resampled)?.periodic_access(
        Duration::from_secs(2),
        |denoised| {
            enabled = !enabled;
            denoised.set_enabled(enabled);
        },
    );

    wav_to_file(denoised, "processed.wav")?;
    Ok(())
}
