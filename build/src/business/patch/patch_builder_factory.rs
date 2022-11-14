use super::PatchBuilder;
use super::PatchType;
use super::kpatch_builder::KernelPatchBuilder;
use super::upatch_builder::UserPatchBuilder;

pub struct PatchBuilderFactory;

impl PatchBuilderFactory {
    pub fn get_patch_builder(patch_type: PatchType, build_root: &str) -> Box<dyn PatchBuilder> {
        match patch_type {
            PatchType::KernelPatch => {
                Box::new(KernelPatchBuilder::new(build_root))
            }
            PatchType::UserPatch => {
                Box::new(UserPatchBuilder::new(build_root))
            },
        }
    }
}
