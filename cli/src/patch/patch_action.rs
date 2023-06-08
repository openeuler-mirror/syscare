use super::patch_status::PatchStatus;

pub trait PatchActionAdapter {
    fn check(&self) -> std::io::Result<()>;
    fn status(&self) -> std::io::Result<PatchStatus>;
    fn apply(&self) -> std::io::Result<()>;
    fn remove(&self) -> std::io::Result<()>;
    fn active(&self) -> std::io::Result<()>;
    fn deactive(&self) -> std::io::Result<()>;
}
