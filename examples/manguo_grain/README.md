# Manguo Grain

A Game-of-Life granular delay, as a VST3 / CLAP plugin for macOS (Apple Silicon).

Audio on the track is recorded into a 16×16 grid of cells. The playhead
walks the columns in `Time`; every live cell holds a grain and plays it when
the playhead passes, and each pass is one generation: with `Life` on, the
grid evolves by Conway's rules and the repeats mutate with it. `Repeats`
sets how much a grain survives each generation; `Digital` / `Analog` / `Tape`
decide how it degrades.

## Download

**[Manguo Grain 0.1.0 — macOS arm64 (VST3 + CLAP)](https://github.com/roy-shiloh/manguo/releases/latest/download/Manguo-Grain-macOS-arm64.zip)**

Install: unzip, then move

- `Manguo Grain.vst3` → `~/Library/Audio/Plug-Ins/VST3/`
- `Manguo Grain.clap` → `~/Library/Audio/Plug-Ins/CLAP/`

and rescan plugins in your DAW (Ableton Live: Settings → Plug-Ins → Rescan).
The bundle is ad-hoc signed; if macOS blocks it, right-click → Open once, or
run `xattr -dr com.apple.quarantine "~/Library/Audio/Plug-Ins/VST3/Manguo Grain.vst3"`.

## Controls

| Section | Control | What it does |
| --- | --- | --- |
| Transport | Run / Freeze | Start the playhead / hold the grid and stop recording |
| | Time, Repeats | Loop length in seconds; survival per generation |
| Grid | cells | Click to place or remove a grain |
| | Digital / Analog / Tape | Degradation per generation |
| | scale | Row → pitch mapping for grains |
| | sweep / up / down | Clear the grid; shift it one row |
| | Life, Stamp, seed | Evolve by Life rules; stamp onsets at the playhead; drop a random pattern |
| Input | Input, Sensitivity | Gain into the delay; gate threshold (0 = closed) |
| | Release, Dry / Wet | Gate release time; equal-power mix |
| Grain | Size, Pitch, Reverse | Grain length in slots; playback rate; reverse probability |
| Motion | Spray, Density, Width | Position jitter; trigger probability; stereo spread |
| Return | Tone, Duck | Low/high-pass on the return; return ducks while input is present |

All parameters are host-automatable; the grid is saved with the preset.

## Build from source

```sh
examples/manguo_grain/bundle.sh --install
```

builds the release `cdylib`, wraps it as `Manguo Grain.vst3` and
`Manguo Grain.clap` under `target/release/bundled/`, and copies them into
`~/Library/Audio/Plug-Ins`. For a quick check without a DAW:

```sh
cargo run --release -p manguo_grain --features standalone --bin manguo_grain_standalone -- --backend coreaudio
```

## How it is put together

- `src/grain_core.rs`, `src/life.rs` — the engine, no UI dependencies.
- `src/lib.rs` — [nice-plug](https://codeberg.org/RustAudio/nice-plug) parameters, VST3/CLAP exports, `process()`.
- `src/editor.rs`, `src/app.rs`, `src/sequencer.rs` — the Makepad panel. The host
  window is a [baseview](https://github.com/RustAudio/baseview) child view;
  Makepad renders into it through `EmbeddedCx`
  (`platform/src/os/apple/macos/macos_embed.rs`).

The full Manguo instrument (synth + voice control + this delay) lives in
[`examples/manguo`](../manguo).
