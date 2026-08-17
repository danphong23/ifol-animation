mod harness;
mod parity_effects;

#[test]
fn run_tc38_kaleidoscope() {
    parity_effects::run(parity_effects::Effect::Kaleidoscope);
}
