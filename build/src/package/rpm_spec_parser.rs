#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[derive(Debug)]
pub struct SpecTag {
    name:  String,
    value: String,
}

impl std::fmt::Display for SpecTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}: {}", self.name, self.value))
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
        f.write_fmt(format_args!("{}{}: {}", self.name, self.id, self.value))
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

        if line.starts_with('#') {
            return None;
        }

        line.find(':').and_then(|spliter_index| {
            let tag_name  = line[..spliter_index].trim().to_string();
            let tag_value = line[spliter_index + 1..].trim().to_string();

            return Some(RpmSpecTag::new_tag(tag_name, tag_value));
        })
    }

    pub fn parse_id_tag(line: &str, tag_prefix: &str) -> Option<RpmSpecTag> {
        if !line.contains(tag_prefix) {
            return None;
        }

        if line.starts_with('#') {
            return None;
        }

        line.find(':').and_then(|spliter_index| {
            let full_tag_name  = line[..spliter_index].trim().to_string();
            let full_tag_value = line[spliter_index + 1..].trim().to_string();

            let tag_prefix_len = tag_prefix.len();
            let full_tag_len   = full_tag_name.len();
            if full_tag_len <= tag_prefix_len {
                return None;
            }

            let tag_name = tag_prefix.to_owned();
            let tag_id   = match full_tag_name[tag_prefix_len..].parse::<usize>() {
                Ok(id) => id,
                Err(_) => return None,
            };
            let tag_value = full_tag_value.trim().to_string();

            return Some(RpmSpecTag::new_id_tag(tag_name, tag_id, tag_value));
        })
    }
}
