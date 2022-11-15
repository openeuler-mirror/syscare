use crate::statics::*;
use crate::util::fs;

pub struct RpmBuildRoot {
    root:       String,
    build:      String,
    build_root: String,
    rpms:       String,
    sources:    String,
    specs:      String,
    srpms:      String,
}

impl RpmBuildRoot {
    pub fn new(work_dir: &str) -> Self {
        let root       = work_dir.to_owned();
        let build      = format!("{}/BUILD", work_dir);
        let build_root = format!("{}/BUILDROOT", work_dir);
        let rpms       = format!("{}/RPMS", work_dir);
        let sources    = format!("{}/SOURCES", work_dir);
        let specs      = format!("{}/SPECS", work_dir);
        let srpms      = format!("{}/SRPMS", work_dir);

        fs::create_dir_all(&root).expect("failed to create rpm build root");
        fs::create_dir(&build).expect("failed to 'BUILD' directory");
        fs::create_dir(&build_root).expect("failed to create 'BUILDROOT' directory");
        fs::create_dir(&rpms).expect("failed to create 'RPMS' directory");
        fs::create_dir(&sources).expect("failed to create 'SOURCES' directory");
        fs::create_dir(&specs).expect("failed to create 'SPECS' directory");
        fs::create_dir(&srpms).expect("failed to create 'SRPMS' directory");

        Self { root, build, build_root, rpms, sources, specs, srpms }
    }

    pub fn get_root_path(&self) -> &str {
        &self.root
    }

    pub fn get_build_path(&self) -> &str {
        &self.build
    }

    pub fn get_build_root_path(&self) -> &str {
        &self.build_root
    }

    pub fn get_source_path(&self) -> &str {
        &self.sources
    }

    pub fn get_spec_path(&self) -> &str {
        &self.specs
    }

    pub fn get_rpm_path(&self) -> &str {
        &self.rpms
    }

    pub fn get_srpm_path(&self) -> &str {
        &self.srpms
    }

    pub fn find_spec_file(&self) -> std::io::Result<String> {
        let spec_file = fs::find_file_ext(self.get_spec_path(), PKG_SPEC_FILE_EXTENSION, false)?;
        Ok(fs::stringtify_path(spec_file))
    }
}

impl std::fmt::Display for RpmBuildRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_root_path())
    }
}
