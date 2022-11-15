use crate::util::fs;

pub struct CliWorkDir {
    work_dir:            String,
    patch_build_root:    String,
    patch_output_dir:    String,
    package_build_root:  String,
}

impl CliWorkDir {
    pub fn new() -> Self {
        let process_id   = std::process::id();
        let process_name = fs::stringtify_path(std::env::current_exe().unwrap().file_name().unwrap());
        let current_dir  = fs::stringtify_path(std::env::current_dir().expect("Get working directory failed"));

        let work_dir            = format!("{}/{}.{}", current_dir, process_name, process_id);
        let patch_build_root    = format!("{}/patch_root", work_dir);
        let patch_output_dir    = format!("{}/patch_output", patch_build_root);
        let package_build_root  = format!("{}/pkg_root", work_dir);

        fs::create_dir_all(&work_dir).expect("Create work directory failed");
        fs::create_dir(&patch_build_root).expect("Create patch build directory");
        fs::create_dir(&patch_output_dir).expect("Create patch output directory failed");
        fs::create_dir(&package_build_root).expect("Create package build directory failed");

        Self {
            work_dir,
            patch_build_root,
            patch_output_dir,
            package_build_root,
        }
    }

    pub fn get_work_dir(&self) -> &str {
        &self.work_dir
    }

    pub fn get_patch_build_root(&self) -> &str {
        &self.patch_build_root
    }

    pub fn get_patch_output_dir(&self) -> &str {
        &self.patch_output_dir
    }

    pub fn get_package_build_root(&self) -> &str {
        &self.package_build_root
    }
}

impl Drop for CliWorkDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(self.get_work_dir()).ok();
    }
}
