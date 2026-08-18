mod harness;
mod scene_effects;

#[test]
fn run_tc46_selective_color() {
    scene_effects::run(scene_effects::Effect::SelectiveColor);
}
