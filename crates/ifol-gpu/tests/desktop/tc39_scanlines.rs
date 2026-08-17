mod harness;
mod parity_effects;

#[test]
fn run_tc39_scanlines() {
    parity_effects::run(parity_effects::Effect::Scanlines);
}
