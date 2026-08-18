mod harness;
mod scene_effects;

#[test]
fn run_tc49_trim_paths() {
    scene_effects::run(scene_effects::Effect::TrimPaths);
}
