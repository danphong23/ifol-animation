mod advanced_compute_62_64;
mod advanced_effects;
mod harness;

#[test]
fn test_tc63_particles_100k() {
    advanced_compute_62_64::run_tc63();
}
