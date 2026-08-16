use super::*;

#[test]
fn texture_version_starts_at_zero_and_marks_changes() {
    let mut registry = ResourceRegistry::new();
    let handle = TextureHandle(11);

    assert_eq!(registry.texture_version(&handle), 0);
    registry.mark_texture_changed(handle);
    assert_eq!(registry.texture_version(&handle), 1);
    registry.mark_texture_changed(handle);
    assert_eq!(registry.texture_version(&handle), 2);
}

#[test]
fn versions_are_typed_and_independent() {
    let mut registry = ResourceRegistry::new();
    registry.mark_texture_changed(TextureHandle(1));
    registry.mark_pipeline_changed(PipelineHandle(1));

    assert_eq!(registry.texture_version(&TextureHandle(1)), 1);
    assert_eq!(registry.pipeline_version(&PipelineHandle(1)), 1);
    assert_eq!(registry.texture_version(&TextureHandle(2)), 0);
    assert_eq!(registry.pipeline_version(&PipelineHandle(2)), 0);
}

#[test]
fn compute_pipeline_versions_are_independent_from_render_pipelines() {
    let mut registry = ResourceRegistry::new();
    registry.mark_pipeline_changed(PipelineHandle(1));
    registry.mark_compute_pipeline_changed(ComputePipelineHandle(1));

    assert_eq!(registry.pipeline_version(&PipelineHandle(1)), 1);
    assert_eq!(
        registry.compute_pipeline_version(&ComputePipelineHandle(1)),
        1
    );
}

#[test]
fn buffer_versions_are_independent_from_texture_versions() {
    let mut registry = ResourceRegistry::new();
    registry.mark_buffer_changed(BufferHandle(1));
    registry.mark_texture_changed(TextureHandle(1));

    assert_eq!(registry.buffer_version(&BufferHandle(1)), 1);
    assert_eq!(registry.texture_version(&TextureHandle(1)), 1);
}
