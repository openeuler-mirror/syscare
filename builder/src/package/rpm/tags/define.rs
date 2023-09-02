#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RpmDefine {
    pub name: String,
    pub value: String,
}

impl std::fmt::Display for RpmDefine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("%define {} {}", self.name, self.value))
    }
}

#[test]
fn test() {
    let define = RpmDefine {
        name: String::from("macro_test"),
        value: String::from("1"),
    };
    println!("RpmMacro::Define\n{}\n", define);
    assert_eq!(define.to_string(), "%define macro_test 1");
}
