use std::str::FromStr;

/// Opaque type for implementing additional `ImageName` features
#[derive(Debug, Clone)]
pub struct OciReference(pub oci_client::Reference);

impl OciReference {
    pub fn id(&self) -> String {
        let id = format!("{}/{}", self.0.registry(), self.0.repository());
        id
    }

    pub fn tag_name(&self) -> String {
        self.0.tag().unwrap_or("latest").to_string()
    }
}

impl FromStr for OciReference {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let reference = oci_client::Reference::from_str(name)?;
        Ok(Self(reference))
    }
}

#[cfg(test)]
mod tests {
    use super::OciReference;
    use anyhow::Result;

    #[test]
    fn test_parse() -> Result<()> {
        let OciReference(image_name) = str::parse("ghcr.io/devcontainers/templates/rust")?;

        assert_eq!(image_name.to_string(), "ghcr.io/devcontainers/templates/rust:latest");

        Ok(())
    }

    #[test]
    fn test_id() -> Result<()> {
        let oci_ref: OciReference = str::parse("ghcr.io/devcontainers/templates/cpp:2")?;

        assert_eq!(oci_ref.id(), "ghcr.io/devcontainers/templates/cpp");

        Ok(())
    }

    #[test]
    fn test_tag_name() -> Result<()> {
        let oci_ref: OciReference = str::parse("github-actions/templates/release:lts")?;

        assert_eq!(oci_ref.tag_name(), "lts");

        Ok(())
    }
}
