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
