use super::kernel_patch::KernelPatchBuilderArguments;
use super::user_patch::UserPatchBuilderArguments;

pub enum PatchBuilderArguments {
    UserPatch(UserPatchBuilderArguments),
    KernelPatch(KernelPatchBuilderArguments),
}
