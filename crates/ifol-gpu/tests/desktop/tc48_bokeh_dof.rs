mod harness;
mod scene_effects;

#[test]
fn run_tc48_bokeh_dof() {
    scene_effects::run(scene_effects::Effect::BokehDof);
}
