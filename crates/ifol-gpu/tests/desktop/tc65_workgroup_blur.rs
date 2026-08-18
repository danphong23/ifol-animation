mod advanced_compute_65_67;
mod advanced_effects;
mod harness;

#[test]
fn test_tc65_workgroup_blur() {
    advanced_compute_65_67::run_tc65();
}
