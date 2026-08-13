pub mod api;
pub mod graph;
pub mod resources;
pub mod render;
pub mod memory;

#[cfg(test)]
mod tests {
    use crate::api::GpuEngineBuilder;

    #[test]
    fn test_headless_initialization() {
        // Initialize logger to see what happens during the test
        let _ = env_logger::builder().is_test(true).try_init();

        // pollster::block_on is used to run async code in synchronous test functions
        let engine_result = pollster::block_on(async {
            GpuEngineBuilder::new().build().await
        });

        assert!(engine_result.is_ok(), "Failed to initialize GpuEngine: {:?}", engine_result.err());
        
        let engine = engine_result.unwrap();
        let caps = engine.capabilities();
        
        // Assert some very basic sane limits that any GPU should have
        assert!(caps.max_texture_dimension_2d >= 2048, "Max texture size is insanely small");
        assert!(caps.max_bind_groups >= 4, "Too few bind groups supported");
        
        println!("Test passed with capabilities: {:?}", caps);
    }
}
