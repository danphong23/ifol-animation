mod harness;
mod scene_effects;

#[test]
fn run_tc45_glassmorphism() {
    scene_effects::run(scene_effects::Effect::Glassmorphism);
}
