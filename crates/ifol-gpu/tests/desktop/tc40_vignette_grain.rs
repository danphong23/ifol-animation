mod harness;
mod parity_effects;

#[test]
fn run_tc40_vignette_grain() {
    parity_effects::run(parity_effects::Effect::VignetteGrain);
}
