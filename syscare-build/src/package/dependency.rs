pub struct PackageDependency {
    requires: HashSet<String>,
    conflicts: HashSet<String>,
    suggests: HashSet<String>,
    recommends: HashSet<String>,
}

impl PackageDependency {

}