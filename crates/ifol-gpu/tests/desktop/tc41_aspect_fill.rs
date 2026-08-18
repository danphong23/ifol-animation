mod harness;
mod simple_asset_effects;

#[test]
fn run_tc41_aspect_fill() {
    simple_asset_effects::run(simple_asset_effects::Effect::AspectFill);
}
