use std::{path::PathBuf, str::FromStr};

use anyhow::{anyhow, Context, Result};
use clap::ArgMatches;

pub trait Parser<'a> {
    fn parse(matches: &ArgMatches<'a>) -> Result<Self>
    where
        Self: Sized;
}

pub trait ArgParser<'a, T> {
    fn parse_arg(matches: &ArgMatches<'a>, arg_name: &str) -> Result<T>;
    fn parse_args(matches: &ArgMatches<'a>, arg_name: &str) -> Result<Vec<T>>;
}

pub struct ArgParserImpl;

impl ArgParserImpl {
    pub fn is_present(matches: &ArgMatches<'_>, arg_name: &str) -> bool {
        matches.is_present(arg_name)
    }
}

impl ArgParser<'_, String> for ArgParserImpl {
    fn parse_arg(matches: &ArgMatches<'_>, arg_name: &str) -> Result<String> {
        let value = matches
            .value_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;
        Ok(value.to_string())
    }

    fn parse_args(matches: &ArgMatches<'_>, arg_name: &str) -> Result<Vec<String>> {
        if !matches.is_present(arg_name) {
            return Ok(vec![]);
        }
        matches
            .values_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))
    }
}

impl ArgParser<'_, bool> for ArgParserImpl {
    fn parse_arg(matches: &ArgMatches<'_>, arg_name: &str) -> Result<bool> {
        Ok(matches.is_present(arg_name))
    }

    fn parse_args(matches: &ArgMatches<'_>, arg_name: &str) -> Result<Vec<bool>> {
        let values = matches
            .values_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;

        let mut args = Vec::new();
        for value in values {
            args.push(
                bool::from_str(&value)
                    .map_err(|e| anyhow!("Failed to parse argument \"{}\", {}", arg_name, e))?,
            );
        }
        Ok(args)
    }
}

impl ArgParser<'_, PathBuf> for ArgParserImpl {
    fn parse_arg(matches: &ArgMatches<'_>, arg_name: &str) -> Result<PathBuf> {
        let value = matches
            .value_of_os(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;
        Ok(PathBuf::from(value))
    }

    fn parse_args(matches: &ArgMatches<'_>, arg_name: &str) -> Result<Vec<PathBuf>> {
        let values = matches
            .values_of_os(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;
        Ok(values.into_iter().map(PathBuf::from).collect())
    }
}

impl ArgParser<'_, usize> for ArgParserImpl {
    fn parse_arg(matches: &ArgMatches<'_>, arg_name: &str) -> Result<usize> {
        let value = matches
            .value_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;

        usize::from_str(&value)
            .map_err(|e| anyhow!("Failed to parse argument \"{}\", {}", arg_name, e))
    }

    fn parse_args(matches: &ArgMatches<'_>, arg_name: &str) -> Result<Vec<usize>> {
        let values = matches
            .values_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;

        let mut args = Vec::new();
        for value in values {
            args.push(
                usize::from_str(&value)
                    .map_err(|e| anyhow!("Failed to parse argument \"{}\", {}", arg_name, e))?,
            );
        }
        Ok(args)
    }
}

impl ArgParser<'_, u32> for ArgParserImpl {
    fn parse_arg(matches: &ArgMatches<'_>, arg_name: &str) -> Result<u32> {
        let value = matches
            .value_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;

        u32::from_str(&value)
            .map_err(|e| anyhow!("Failed to parse argument \"{}\", {}", arg_name, e))
    }

    fn parse_args(matches: &ArgMatches<'_>, arg_name: &str) -> Result<Vec<u32>> {
        let values = matches
            .values_of_lossy(arg_name)
            .with_context(|| format!("Argument \"{}\" is not present", arg_name))?;

        let mut args = Vec::new();
        for value in values {
            args.push(
                u32::from_str(&value)
                    .map_err(|e| anyhow!("Failed to parse argument \"{}\", {}", arg_name, e))?,
            );
        }
        Ok(args)
    }
}

#[test]
fn test_args_parser() {
    use clap::{clap_app, crate_description, crate_name, crate_version};

    const DEFAULT_WORK_DIR: &str = "/var/run/syscare";

    let test_match = clap_app!(syscare_cli =>
        (name: crate_name!())
        (version: crate_version!())
        (about: crate_description!())
        (set_term_width: 120)
        (@arg work_dir: short("w") long("work-dir") value_name("WORK_DIR") +takes_value default_value(DEFAULT_WORK_DIR) "Path for working directory")
        (@arg verbose: short("v") long("verbose") "Provide more detailed info")
        (@arg name: short("n") long("name") value_name("syscarename") +takes_value default_value("syscaretest") "test for string")
        (@arg size: short("s") long("size") value_name("size") +takes_value default_value("2024") "test for usize")
        (@subcommand argsstring =>
            (about: "test args string")
            (@arg identifier: value_name("IDENTIFIER") +takes_value default_value("test args string") +multiple +required "Patch identifier")
        )
        (@subcommand argspath =>
            (about: "test args path")
            (@arg identifier: value_name("IDENTIFIER") +takes_value default_value("/var/log/testsyscare") +multiple +required "Patch identifier")
        )
        (@subcommand argsbool =>
            (about: "test args bool")
            (@arg identifier: value_name("IDENTIFIER") +takes_value default_value("true") +multiple +required "Patch identifier")
        )
        (@subcommand argssize =>
            (about: "test args size")
            (@arg identifier: value_name("IDENTIFIER") +takes_value default_value("2024") +multiple +required "Patch identifier")
        )
    );
    let mut arg:Vec<&str> = Vec::new();
    arg.push("testsyscare");
    arg.push("argsstring");
    let mut test = test_match.clone().get_matches_from(&arg);

    let arg_path:Result<PathBuf> = ArgParserImpl::parse_arg(&test, "work_dir");
    println!("test dir {:#?}", arg_path);
    assert!(arg_path.is_ok());

    let arg_bool:Result<bool> = ArgParserImpl::parse_arg(&test, "verbose");
    println!("test bool {:#?}", arg_bool);
    assert!(arg_bool.is_ok());

    let arg_string:Result<String> = ArgParserImpl::parse_arg(&test, "name");
    println!("test string {:#?}", arg_string);
    assert!(arg_string.is_ok());

    let arg_usize:Result<usize> = ArgParserImpl::parse_arg(&test, "size");
    println!("test usize {:#?}", arg_usize);
    assert!(arg_usize.is_ok());

   if let Some(cmd_match) = &test.subcommand_matches("argsstring") {
        let args_string:Result<Vec<String>> = ArgParserImpl::parse_args(cmd_match, "identifier");
        println!("test arg string  {:#?}", args_string);
        assert!(args_string.is_ok());
    } else {
        panic!("No argsstring found!");
    }

    arg.pop();
    arg.push("argspath");
    test = test_match.clone().get_matches_from(&arg);
    if let Some(cmd_match) = &test.subcommand_matches("argspath") {
        let args_path:Result<Vec<PathBuf>> = ArgParserImpl::parse_args(cmd_match, "identifier");
        println!("test arg path  {:#?}", args_path);
        assert!(args_path.is_ok());
    } else {
        panic!("No argspath found!");
    }
    arg.pop();
    arg.push("argsbool");
    test = test_match.clone().get_matches_from(&arg);
    if let Some(cmd_match) = &test.subcommand_matches("argsbool") {
        let args_bool:Result<Vec<bool>> = ArgParserImpl::parse_args(cmd_match, "identifier");
        println!("test arg bool  {:#?}", args_bool);
        assert!(args_bool.is_ok());
    } else {
        panic!("No argsbool found!");
    }

    arg.pop();
    arg.push("argssize");
    test = test_match.clone().get_matches_from(&arg);
    if let Some(cmd_match) = &test.subcommand_matches("argssize") {
        let args_size:Result<Vec<usize>> = ArgParserImpl::parse_args(cmd_match, "identifier");
        println!("test arg size  {:#?}", args_size);
        assert!(args_size.is_ok());
    } else {
        panic!("No argssize found!");
    }
}
