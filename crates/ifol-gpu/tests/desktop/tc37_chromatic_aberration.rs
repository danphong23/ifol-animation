mod harness;
mod parity_effects;

#[test]
fn run_tc37_chromatic_aberration() {
    parity_effects::run(parity_effects::Effect::ChromaticAberration);
}
