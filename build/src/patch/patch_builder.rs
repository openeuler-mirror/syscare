use super::PatchBuilderOptions;

pub trait PatchBuilder {
    fn build_patch(&self, options: PatchBuilderOptions) -> std::io::Result<()>;
}
