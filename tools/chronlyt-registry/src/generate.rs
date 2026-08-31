use crate::{RegistryError, RegistryResult, RegistrySourceV1};
use chronlyt_plugin_contracts::{
    CatalogArtifactV1, PluginCatalogEntryV1, PluginCatalogV1, limits::MAX_CATALOG_BYTES,
};
use chronlyt_plugin_validator::{inspect_package, validate_component_contract};
use semver::Version;
use std::{
    collections::BTreeMap,
    io::Write,
    time::{Duration, Instant},
};

pub const MAX_TRANSFER_BYTES: u64 = 256 * 1024 * 1024;
pub const GENERATION_TIMEOUT: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Copy)]
pub struct FetchBudget {
    pub max_bytes: usize,
    pub deadline: Instant,
}

pub trait ArtifactFetcher {
    fn fetch(&mut self, url: &str, budget: FetchBudget) -> RegistryResult<Vec<u8>>;
}

pub fn generate_catalog(
    source: &RegistrySourceV1,
    previous: &PluginCatalogV1,
    fetcher: &mut dyn ArtifactFetcher,
) -> RegistryResult<Vec<u8>> {
    let started = Instant::now();
    source.validate()?;
    previous.validate()?;
    let previous: BTreeMap<_, _> = previous
        .plugins
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();
    let mut records: Vec<_> = source.records.iter().collect();
    records.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    let mut plugins = Vec::new();
    let mut entry_bytes = EntryCounter(0);
    let mut transferred = 0u64;
    for record in records {
        if started.elapsed() >= GENERATION_TIMEOUT {
            return Err(RegistryError::Limit("generation deadline"));
        }
        let version = Version::parse(&record.release.version)?;
        let prior = previous.get(record.id.as_str());
        if let Some(prior) = prior {
            if prior.created_at != record.created_at {
                return Err(RegistryError::Invalid("created timestamp replacement"));
            }
            let old_version = Version::parse(&prior.version)?;
            if version != old_version && !version.cmp_precedence(&old_version).is_gt() {
                return Err(RegistryError::Invalid("version must advance"));
            }
        }
        let budget = FetchBudget {
            max_bytes: (MAX_TRANSFER_BYTES - transferred) as usize,
            deadline: started + GENERATION_TIMEOUT,
        };
        let bytes = fetcher.fetch(&record.release.artifact_url, budget)?;
        transferred = transferred
            .checked_add(bytes.len() as u64)
            .ok_or(RegistryError::Limit("aggregate transfer"))?;
        if transferred > MAX_TRANSFER_BYTES {
            return Err(RegistryError::Limit("aggregate transfer"));
        }
        let package = inspect_package(&bytes)?;
        let manifest = package.manifest();
        if manifest.id != record.id || Version::parse(&manifest.version)? != version {
            return Err(RegistryError::Invalid("package release identity"));
        }
        if let Some(prior) = prior
            && version == Version::parse(&prior.version)?
            && (prior.artifact.sha256 != package.summary().sha256
                || prior.artifact.size != package.summary().size)
        {
            return Err(RegistryError::Invalid("same-version artifact replacement"));
        }
        validate_component_contract(package.component())?;
        if started.elapsed() >= GENERATION_TIMEOUT {
            return Err(RegistryError::Limit("generation deadline"));
        }
        let entry = PluginCatalogEntryV1 {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            summary: record.summary.clone(),
            description: manifest.description.clone(),
            author: manifest.author.clone(),
            publisher: record.publisher.clone(),
            repository_url: record.repository_url.clone(),
            license: record.license.clone(),
            categories: record.categories.clone(),
            tags: record.tags.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            version: manifest.version.clone(),
            chronlyt_compatibility: manifest.compatibility.chronlyt.clone(),
            api_version: manifest.api_version,
            artifact: CatalogArtifactV1 {
                url: record.release.artifact_url.clone(),
                sha256: package.summary().sha256.clone(),
                size: package.summary().size,
            },
            permissions: manifest.permissions.clone(),
            changelog: record.changelog.clone(),
            downloads: None,
            stars: None,
            verified: false,
            official: false,
        };
        // Compact JSON is a lower bound on the final pretty output. Stop retaining
        // entries (and fetching more artifacts) as soon as that budget is spent.
        serde_json::to_writer(&mut entry_bytes, &entry)
            .map_err(|_| RegistryError::Limit("catalog bytes"))?;
        plugins.push(entry);
    }
    let catalog = PluginCatalogV1 {
        schema_version: 1,
        generated_at: source.metadata.generated_at.clone(),
        plugins,
    };
    catalog.validate()?;
    let mut output = CatalogWriter(Vec::new());
    serde_json::to_writer_pretty(&mut output, &catalog)
        .map_err(|_| RegistryError::Limit("catalog bytes"))?;
    output.0.push(b'\n');
    Ok(output.0)
}

struct EntryCounter(usize);
impl Write for EntryCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_CATALOG_BYTES.saturating_sub(self.0) {
            return Err(std::io::Error::other("catalog limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CatalogWriter(Vec<u8>);
impl Write for CatalogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > (MAX_CATALOG_BYTES - 1).saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("catalog limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
