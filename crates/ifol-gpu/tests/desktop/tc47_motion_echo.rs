mod harness;
mod scene_effects;

#[test]
fn run_tc47_motion_echo() {
    scene_effects::run(scene_effects::Effect::MotionEcho);
}
