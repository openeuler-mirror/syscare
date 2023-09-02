use chrono::{DateTime, Local};

#[derive(Debug)]
pub struct RpmChangeLog {
    pub date: DateTime<Local>,
    pub author: String,
    pub version: String,
    pub records: Vec<String>,
}

impl std::fmt::Display for RpmChangeLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "* {} {} - {}",
            self.date.format("%a %b %d %Y"),
            self.author,
            self.version,
        )?;
        for line in &self.records {
            writeln!(f)?;
            write!(f, "- {}", line)?;
        }

        Ok(())
    }
}

#[test]
fn test() {
    use std::str::FromStr;

    let change_log = RpmChangeLog {
        date: DateTime::from_str("2023-09-01T00:00:00Z").unwrap(),
        author: String::from("syscare"),
        version: String::from("1.0.0"),
        records: vec![String::from("test record")],
    };
    println!("RpmChangeLog::new()\n{}\n", change_log);
    assert_eq!(
        change_log.to_string(),
        "* Fri Sep 01 2023 syscare - 1.0.0\n- test record"
    );
}
