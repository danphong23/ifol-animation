mod harness;
mod simple_asset_effects;

#[test]
fn run_tc44_anamorphic_flare() {
    simple_asset_effects::run(simple_asset_effects::Effect::AnamorphicFlare);
}
