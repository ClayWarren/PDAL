use std::fmt;
use std::str::FromStr;

/// Supported PDAL plugin families.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PluginKind {
    Reader,
    Writer,
    Filter,
    Kernel,
}

impl PluginKind {
    pub fn singular(self) -> &'static str {
        match self {
            Self::Reader => "reader",
            Self::Writer => "writer",
            Self::Filter => "filter",
            Self::Kernel => "kernel",
        }
    }

    pub fn stage_prefix(self) -> &'static str {
        match self {
            Self::Reader => "readers",
            Self::Writer => "writers",
            Self::Filter => "filters",
            Self::Kernel => "kernels",
        }
    }
}

impl fmt::Display for PluginKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.singular())
    }
}

impl FromStr for PluginKind {
    type Err = PluginKindParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "reader" => Ok(Self::Reader),
            "writer" => Ok(Self::Writer),
            "filter" => Ok(Self::Filter),
            "kernel" => Ok(Self::Kernel),
            _ => Err(PluginKindParseError),
        }
    }
}

/// Error returned when a plugin kind string is not recognized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginKindParseError;

impl fmt::Display for PluginKindParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown plugin kind")
    }
}

impl std::error::Error for PluginKindParseError {}

/// User-visible plugin metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub link: String,
}

impl PluginInfo {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        link: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            link: link.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_kind_strings_match_pdal_names() {
        assert_eq!(PluginKind::Reader.singular(), "reader");
        assert_eq!(PluginKind::Reader.stage_prefix(), "readers");
        assert_eq!(PluginKind::Writer.singular(), "writer");
        assert_eq!(PluginKind::Writer.stage_prefix(), "writers");
        assert_eq!(PluginKind::Filter.singular(), "filter");
        assert_eq!(PluginKind::Filter.stage_prefix(), "filters");
        assert_eq!(PluginKind::Kernel.singular(), "kernel");
        assert_eq!(PluginKind::Kernel.stage_prefix(), "kernels");
    }

    #[test]
    fn plugin_kind_parses_singular_names_only() {
        assert_eq!("reader".parse::<PluginKind>().unwrap(), PluginKind::Reader);
        assert_eq!("writer".parse::<PluginKind>().unwrap(), PluginKind::Writer);
        assert_eq!("filter".parse::<PluginKind>().unwrap(), PluginKind::Filter);
        assert_eq!("kernel".parse::<PluginKind>().unwrap(), PluginKind::Kernel);
        assert!("readers".parse::<PluginKind>().is_err());
    }

    #[test]
    fn plugin_info_preserves_metadata() {
        let info = PluginInfo::new(
            "readers.example",
            "Example reader",
            "https://pdal.io/stages/readers.example.html",
        );

        assert_eq!(info.name, "readers.example");
        assert_eq!(info.description, "Example reader");
        assert_eq!(info.link, "https://pdal.io/stages/readers.example.html");
    }
}
