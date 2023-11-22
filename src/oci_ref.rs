use std::str::FromStr;

use ocipkg::ImageName;

/// Opaque type for implementing additional ImageName features
#[derive(Debug, Clone)]
pub struct OciReference(pub ImageName);

impl OciReference {
    pub fn id(&self) -> String {
        let id = format!("{}/{}", self.0.hostname, self.0.name);
        id
    }

    pub fn tag_name(&self) -> String {
        self.0.reference.to_string()
    }
}

impl FromStr for OciReference {
    type Err = ocipkg::error::Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        ImageName::parse(name)
        .map(OciReference)
    }
}
