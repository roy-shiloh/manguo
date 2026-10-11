//! Standalone host for the plugin (`--features standalone`). Useful for
//! runtime checks of the editor without a DAW: `--backend dummy` skips audio I/O.
use manguo_grain::ManguoGrain;
use nice_plug::prelude::*;

fn main() {
    nice_export_standalone::<ManguoGrain>();
}
