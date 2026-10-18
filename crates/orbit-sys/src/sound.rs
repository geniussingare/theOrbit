//! The optional "select" sound: one soft, short tick when an item is chosen.
//!
//! It is synthesised once, cached as a tiny WAV next to the config, and
//! played with the OS's own player, so no audio library is linked.

use std::path::PathBuf;

const RATE: u32 = 44_100;
const SECONDS: f32 = 0.09;

/// A gentle tick: a ~1.2 kHz sine with a slight downward glide, a 2 ms
/// attack (no click) and a fast exponential decay, plus a quiet octave below
/// for warmth.
pub fn tick_wav() -> Vec<u8> {
    let n = (RATE as f32 * SECONDS) as usize;
    let mut phase = 0.0f32;
    let pcm: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let freq = 1_150.0 + 250.0 * (-t / 0.012).exp();
            phase += std::f32::consts::TAU * freq / RATE as f32;
            let env = (t / 0.002).min(1.0) * (-t / 0.018).exp();
            let s = (phase.sin() + 0.35 * (phase * 0.5).sin()) * env;
            (s * 0.32 * i16::MAX as f32) as i16
        })
        .collect();
    wav_bytes(&pcm, 1)
}

/// Wraps interleaved 16-bit samples in a WAV header.
fn wav_bytes(pcm: &[i16], channels: u16) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let block = channels * 2;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * block as u32).to_le_bytes()); // byte rate
    wav.extend_from_slice(&block.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        wav.extend_from_slice(&s.to_le_bytes());
    }
    wav
}

fn cached_wav() -> Option<PathBuf> {
    let dir = crate::paths::config_dir();
    // Versioned so an improved sound replaces older cached clips.
    let path = dir.join("select-v1.wav");
    if !path.exists() {
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, tick_wav()).ok()?;
        // Clips from earlier versions (the thunder) are no longer used.
        for old in ["thunder.wav", "thunder-v2.wav", "thunder-v3.wav"] {
            let _ = std::fs::remove_file(dir.join(old));
        }
    }
    Some(path)
}

/// Plays the select tick without blocking.
pub fn play_select() {
    let Some(path) = cached_wav() else { return };
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("afplay").arg(&path).spawn();
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::core::{HSTRING, PCWSTR};
        use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_FILENAME, SND_NODEFAULT};
        let wide = HSTRING::from(path.as_os_str());
        let _ = PlaySoundW(PCWSTR(wide.as_ptr()), None, SND_FILENAME | SND_ASYNC | SND_NODEFAULT);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let p = path.display().to_string();
        let _ = std::process::Command::new("sh")
            .args(["-c", "paplay \"$0\" 2>/dev/null || aplay -q \"$0\""])
            .arg(p)
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_is_short_valid_wav() {
        let wav = tick_wav();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 1, "mono");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(wav.len(), 44 + data_len);
        assert!(data_len / 2 < RATE as usize / 5, "under 200 ms");
        // Soft: peak well below full scale.
        let peak = wav[44..].chunks(2).map(|s| i16::from_le_bytes([s[0], s[1]]).unsigned_abs()).max().unwrap();
        assert!(peak > 2_000 && peak < 16_000);
    }
}
