use crate::util::fs;

const WORK_DIR_PREFIX: &str = "syscare/patch-build";

pub struct CliWorkDir {
    work_dir:            String,
    patch_build_root:    String,
    patch_output_dir:    String,
    package_build_root:  String,
    package_extract_dir: String,
}

impl CliWorkDir {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().canonicalize().expect("Get temporary directory failed");
        let work_dir = format!("{}/{}.{}", temp_dir.display(), WORK_DIR_PREFIX, std::process::id());
        let patch_build_root = format!("{}/patch_root", work_dir);
        let patch_output_dir = format!("{}/patch_output", patch_build_root);
        let package_build_root = format!("{}/pkg_root", work_dir);
        let package_extract_dir = format!("{}/pkg_extract", work_dir);

        fs::create_dir_all(&work_dir).expect("Create work directory failed");
        fs::create_dir(&patch_build_root).expect("Create patch build directory");
        fs::create_dir(&patch_output_dir).expect("Create patch output directory failed");
        fs::create_dir(&package_build_root).expect("Create package build directory failed");
        fs::create_dir(&package_extract_dir).expect("Create package extract directory failed");

        Self {
            work_dir,
            patch_build_root,
            patch_output_dir,
            package_build_root,
            package_extract_dir,
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

    pub fn get_package_extract_dir(&self) -> &str {
        &self.package_extract_dir
    }
}

impl Drop for CliWorkDir {
    fn drop(&mut self) {
        // std::fs::remove_dir_all(self.get_work_dir()).ok();
    }
}
