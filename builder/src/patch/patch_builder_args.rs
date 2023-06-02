use super::user_patch::UserPatchBuilderArguments;
use super::kernel_patch::KernelPatchBuilderArguments;

pub enum PatchBuilderArguments {
    UserPatch(UserPatchBuilderArguments),
    KernelPatch(KernelPatchBuilderArguments),
}
