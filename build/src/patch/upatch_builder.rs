use super::{PatchBuilder, PatchBuilderOptions};

pub struct UserPatchBuilder {
    _build_root: String
}

impl UserPatchBuilder {
    pub fn new(_build_root: &str) -> Self {
        unimplemented!("User patch builder is not implemented");
    }
}

impl PatchBuilder for UserPatchBuilder {
    fn build_patch(&self, _options: PatchBuilderOptions) -> std::io::Result<()> {
        unreachable!();
    }
}
