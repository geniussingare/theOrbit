//! Writes the select tick: `cargo run -p orbit-sys --example tick -- out.wav`
fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "tick.wav".into());
    std::fs::write(&out, orbit_sys::sound::tick_wav()).unwrap();
}
