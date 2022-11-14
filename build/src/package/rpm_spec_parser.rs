use std::str::FromStr;

const SPEC_TAG_SPLITER: char = ':';

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[derive(Debug)]
pub struct SpecTag {
    name:  String,
    value: String,
}

impl std::fmt::Display for SpecTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{} {}", self.name, SPEC_TAG_SPLITER, self.value))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[derive(Debug)]
pub struct SpecIdTag {
    name:  String,
    id:    usize,
    value: String,
}

impl std::fmt::Display for SpecIdTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}{} {}", self.name, self.id, SPEC_TAG_SPLITER, self.value))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[derive(Debug)]
pub enum RpmSpecTag {
    Tag(SpecTag),
    IdTag(SpecIdTag)
}

impl RpmSpecTag {
    pub fn new_tag(name: String, value: String) -> Self {
        Self::Tag(SpecTag { name, value})
    }

    pub fn new_id_tag(name: String, id: usize, value: String) -> Self {
        Self::IdTag(SpecIdTag { name, id, value})
    }

    pub fn get_name(&self) -> &str {
        match self {
            Self::Tag(tag) => &tag.name,
            Self::IdTag(tag) => &tag.name
        }
    }

    pub fn get_id(&self) -> Option<usize> {
        match self {
            Self::Tag(_)     => None,
            Self::IdTag(tag) => Some(tag.id)
        }
    }

    pub fn get_value(&self) -> &str {
        match self {
            Self::Tag(tag)   => &tag.value,
            Self::IdTag(tag) => &tag.value,
        }
    }
}

impl std::fmt::Display for RpmSpecTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpmSpecTag::Tag(tag)   => f.write_fmt(format_args!("{}", tag)),
            RpmSpecTag::IdTag(tag) => f.write_fmt(format_args!("{}", tag)),
        }
    }
}

pub struct RpmSpecParser;

impl RpmSpecParser {
    pub fn parse_tag(line: &str, tag_name: &str) -> Option<RpmSpecTag> {
        if !line.contains(tag_name) {
            return None;
        }

        let tag_info = line.split(SPEC_TAG_SPLITER).collect::<Vec<&str>>();
        if tag_info.len() != 2 {
            return None;
        }

        let tag_name  = tag_info[0].trim().to_owned();
        let tag_value = tag_info[1].trim().to_owned();

        Some(RpmSpecTag::new_tag(tag_name, tag_value))
    }

    pub fn parse_parse_id_tag(line: &str, tag_prefix: &str) -> Option<RpmSpecTag> {
        if !line.contains(tag_prefix) {
            return None;
        }

        let tag_info = line.split(SPEC_TAG_SPLITER).collect::<Vec<&str>>();
        if tag_info.len() != 2 {
            return None;
        }

        let full_tag_name = tag_info[0].trim().to_owned();
        let source_id_idx = tag_prefix.len();

        if full_tag_name.len() < source_id_idx {
            return None;
        }

        let tag_name  = tag_prefix.to_owned();
        let tag_id    = match usize::from_str(&full_tag_name[source_id_idx..]) {
            Ok(id) => id,
            Err(_) => return None,
        };
        let tag_value = tag_info[1].trim().to_owned();

        Some(RpmSpecTag::new_id_tag(tag_name, tag_id, tag_value))
    }
}
