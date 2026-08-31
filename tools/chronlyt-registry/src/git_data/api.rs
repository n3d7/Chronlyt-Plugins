//! Fixed-repository PR/Git API reads, never clone URLs or contribution code.
use super::{
    ACQUISITION_TIMEOUT, BlobReader, MAX_REQUESTS, MAX_TREE_BYTES, TreeEntry, catalog_from_tree,
    regular, source_from_tree, validate_revision, validate_tree,
};
use crate::{
    RegistryError, RegistryResult, RegistrySourceV1,
    source::MAX_SOURCE_BYTES,
    transport::{
        HttpResponse, HttpTransport, credential_free_client, read_bounded_body, response_parts,
    },
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{
    blocking::Client,
    header::{AUTHORIZATION, HeaderValue},
};
use serde::{Deserialize, de::DeserializeOwned};
use std::time::{Duration, Instant};

const API: &str = "https://api.github.com/repos/n3d7/Chronlyt-Plugins/";
const REPOSITORY: &str = "n3d7/Chronlyt-Plugins";
const MAX_API_BYTES: usize = 16 * 1024 * 1024;
const MAX_PR_NUMBER: u64 = i32::MAX as u64;

pub struct PrDataSnapshot {
    pub source: RegistrySourceV1,
    pub catalog_bytes: Vec<u8>,
    pub head_sha: String,
    base_sha: String,
    pr_number: u64,
}
impl PrDataSnapshot {
    pub fn base_sha(&self) -> &str {
        &self.base_sha
    }
    pub fn pr_number(&self) -> u64 {
        self.pr_number
    }
}

pub struct ApiTransport {
    client: Client,
    authorization: Option<HeaderValue>,
}
impl ApiTransport {
    fn from_environment() -> RegistryResult<Self> {
        // Only the dedicated, short-lived read-only workflow token is accepted.
        // Never consult GH_TOKEN, GITHUB_TOKEN, netrc or local account storage.
        let authorization = std::env::var("CHRONLYT_REGISTRY_GITHUB_TOKEN")
            .ok()
            .map(|token| {
                if token.is_empty() || token.len() > 4096 {
                    return Err(RegistryError::Invalid("API credential"));
                }
                let mut value = HeaderValue::from_str(&format!("Bearer {token}"))
                    .map_err(|_| RegistryError::Invalid("API credential"))?;
                value.set_sensitive(true);
                Ok(value)
            })
            .transpose()?;
        Ok(Self {
            client: credential_free_client()?,
            authorization,
        })
    }
}
impl HttpTransport for ApiTransport {
    fn get(&mut self, url: &str, timeout: Duration) -> RegistryResult<HttpResponse> {
        let path = url
            .strip_prefix(API)
            .ok_or(RegistryError::Invalid("API endpoint"))?;
        validate_endpoint(path)?;
        let mut request = self
            .client
            .get(url)
            .timeout(timeout)
            .header("Accept", "application/vnd.github+json")
            .header("Accept-Encoding", "identity")
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(value) = &self.authorization {
            request = request.header(AUTHORIZATION, value.clone());
        }
        // Client redirects are disabled; credentials cannot follow a Location.
        response_parts(request.send().map_err(|_| RegistryError::Transport)?)
    }
}

fn validate_pr_number(value: u64) -> RegistryResult<()> {
    if value == 0 || value > MAX_PR_NUMBER {
        return Err(RegistryError::Invalid("PR number"));
    }
    Ok(())
}
fn validate_endpoint(path: &str) -> RegistryResult<()> {
    if path == "git/ref/heads/main" {
        return Ok(());
    }
    if let Some(number) = path.strip_prefix("pulls/") {
        let number = number
            .parse::<u64>()
            .map_err(|_| RegistryError::Invalid("PR number"))?;
        validate_pr_number(number)?;
        if path != format!("pulls/{number}") {
            return Err(RegistryError::Invalid("API endpoint"));
        }
        return Ok(());
    }
    for prefix in ["git/commits/", "git/trees/", "git/blobs/"] {
        if let Some(revision) = path.strip_prefix(prefix) {
            return validate_revision(revision);
        }
    }
    Err(RegistryError::Invalid("API endpoint"))
}

/// Keep this reader alive through final freshness verification so all API
/// reads share their request/encoded/decoded/deadline budgets.
pub struct PrDataReader<T = ApiTransport> {
    transport: T,
    deadline: Instant,
    requests: usize,
    encoded: usize,
    decoded: usize,
}
impl PrDataReader<ApiTransport> {
    pub fn new() -> RegistryResult<Self> {
        Ok(Self::with_transport(ApiTransport::from_environment()?))
    }
}
impl<T: HttpTransport> PrDataReader<T> {
    pub fn with_transport(transport: T) -> Self {
        Self {
            transport,
            deadline: Instant::now() + ACQUISITION_TIMEOUT,
            requests: 0,
            encoded: 0,
            decoded: 0,
        }
    }
    fn json<V: DeserializeOwned>(&mut self, path: &str, cap: usize) -> RegistryResult<V> {
        validate_endpoint(path)?;
        if cap == 0 || self.encoded >= MAX_API_BYTES {
            return Err(RegistryError::Limit("API response bytes"));
        }
        self.requests += 1;
        if self.requests > MAX_REQUESTS {
            return Err(RegistryError::Limit("API requests"));
        }
        let deadline = self.deadline.min(Instant::now() + Duration::from_secs(30));
        let timeout = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(RegistryError::Limit("API deadline"))?;
        let response = self.transport.get(&format!("{API}{path}"), timeout)?;
        if response.status != 200 {
            return Err(RegistryError::Transport);
        }
        let bytes = read_bounded_body(
            response,
            cap.min(MAX_API_BYTES.saturating_sub(self.encoded)),
            deadline,
        )?;
        self.encoded += bytes.len();
        Ok(serde_json::from_slice(&bytes)?)
    }
    fn identity(&mut self, number: u64, expected_head: &str) -> RegistryResult<String> {
        validate_pr_number(number)?;
        validate_revision(expected_head)?;
        let pr: PullRequest = self.json(&format!("pulls/{number}"), MAX_TREE_BYTES)?;
        validate_revision(&pr.head.sha)?;
        validate_revision(&pr.base.sha)?;
        if pr.number != number
            || pr.state != "open"
            || pr.head.sha != expected_head
            || pr.base.branch != "main"
            || pr.base.repo.full_name != REPOSITORY
        {
            return Err(RegistryError::Invalid("PR identity or target changed"));
        }
        let main: GitReference = self.json("git/ref/heads/main", MAX_TREE_BYTES)?;
        validate_revision(&main.object.sha)?;
        if main.reference != "refs/heads/main"
            || main.object.kind != "commit"
            || main.object.sha != pr.base.sha
        {
            return Err(RegistryError::Invalid("PR base is stale"));
        }
        Ok(main.object.sha)
    }
    pub fn read(&mut self, number: u64, expected_head: &str) -> RegistryResult<PrDataSnapshot> {
        let base_sha = self.identity(number, expected_head)?;
        let commit: GitCommit =
            self.json(&format!("git/commits/{expected_head}"), MAX_TREE_BYTES)?;
        if commit.sha != expected_head {
            return Err(RegistryError::Invalid("commit identity"));
        }
        validate_revision(&commit.tree.sha)?;
        let root = self.tree(&commit.tree.sha)?;
        let catalog_bytes =
            catalog_from_tree(self, &root)?.ok_or(RegistryError::Invalid("catalog missing"))?;
        let source = source_from_tree(self, &root)?;
        Ok(PrDataSnapshot {
            source,
            catalog_bytes,
            head_sha: expected_head.into(),
            base_sha,
            pr_number: number,
        })
    }
    pub fn verify_freshness(
        &mut self,
        snapshot: &PrDataSnapshot,
        trusted_revision: &str,
    ) -> RegistryResult<()> {
        validate_revision(trusted_revision)?;
        if snapshot.base_sha != trusted_revision {
            return Err(RegistryError::Invalid("trusted main revision is stale"));
        }
        let base = self.identity(snapshot.pr_number, &snapshot.head_sha)?;
        if base != snapshot.base_sha {
            return Err(RegistryError::Invalid("PR base changed"));
        }
        Ok(())
    }
}

impl<T: HttpTransport> BlobReader for PrDataReader<T> {
    fn tree(&mut self, revision: &str) -> RegistryResult<Vec<TreeEntry>> {
        validate_revision(revision)?;
        // No recursive parameter, pagination URL, archive or fork endpoint.
        let response: GitTree = self.json(&format!("git/trees/{revision}"), MAX_TREE_BYTES)?;
        if response.sha != revision || response.truncated {
            return Err(RegistryError::Invalid("tree identity or truncation"));
        }
        let entries = response
            .tree
            .into_iter()
            .map(|entry| TreeEntry {
                name: entry.path,
                mode: entry.mode,
                kind: entry.kind,
                sha: entry.sha,
                size: entry.size,
            })
            .collect::<Vec<_>>();
        validate_tree(&entries)?;
        Ok(entries)
    }
    fn blob(&mut self, entry: &TreeEntry, max_bytes: usize) -> RegistryResult<Vec<u8>> {
        regular(entry)?;
        let cap = max_bytes.min(MAX_SOURCE_BYTES.saturating_sub(self.decoded));
        let size = entry
            .size
            .filter(|size| *size <= cap as u64)
            .ok_or(RegistryError::Limit("blob bytes"))? as usize;
        let encoded_size = size.div_ceil(3) * 4;
        // Allow GitHub's folded base64 and JSON metadata, but bound the raw
        // response before JSON parsing or decoding allocates its buffers.
        let response: GitBlob = self.json(
            &format!("git/blobs/{}", entry.sha),
            encoded_size.saturating_mul(2).saturating_add(4096),
        )?;
        if response.sha != entry.sha
            || response.size != size as u64
            || response.encoding != "base64"
        {
            return Err(RegistryError::Invalid("blob identity or encoding"));
        }
        let encoded = response
            .content
            .bytes()
            .filter(|b| !matches!(b, b'\n' | b'\r'))
            .collect::<Vec<_>>();
        if encoded.len() != encoded_size {
            return Err(RegistryError::Invalid("blob size disagreement"));
        }
        let mut decoded = vec![0; size];
        let written = STANDARD
            .decode_slice(&encoded, &mut decoded)
            .map_err(|_| RegistryError::Invalid("blob encoding"))?;
        if written != size {
            return Err(RegistryError::Invalid("blob size disagreement"));
        }
        self.decoded += written;
        Ok(decoded)
    }
}

/// Convenience acquisition check. Command orchestration uses PrDataReader
/// directly and verifies freshness again after catalog generation.
pub fn read_source_pr(number: u64, head: &str) -> RegistryResult<PrDataSnapshot> {
    let mut reader = PrDataReader::new()?;
    let snapshot = reader.read(number, head)?;
    reader.verify_freshness(&snapshot, snapshot.base_sha())?;
    Ok(snapshot)
}

#[derive(Deserialize)]
struct Sha {
    sha: String,
}
#[derive(Deserialize)]
struct Repo {
    full_name: String,
}
#[derive(Deserialize)]
struct Base {
    sha: String,
    #[serde(rename = "ref")]
    branch: String,
    repo: Repo,
}
#[derive(Deserialize)]
struct PullRequest {
    number: u64,
    state: String,
    head: Sha,
    base: Base,
}
#[derive(Deserialize)]
struct RefObject {
    sha: String,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Deserialize)]
struct GitReference {
    #[serde(rename = "ref")]
    reference: String,
    object: RefObject,
}
#[derive(Deserialize)]
struct GitCommit {
    sha: String,
    tree: Sha,
}
#[derive(Deserialize)]
struct GitTree {
    sha: String,
    truncated: bool,
    tree: Vec<ApiTreeEntry>,
}
#[derive(Deserialize)]
struct ApiTreeEntry {
    path: String,
    mode: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
}
#[derive(Deserialize)]
struct GitBlob {
    sha: String,
    size: u64,
    encoding: String,
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    struct NoRequest;
    impl HttpTransport for NoRequest {
        fn get(&mut self, _: &str, _: Duration) -> RegistryResult<HttpResponse> {
            panic!("must reject before requesting")
        }
    }
    #[test]
    fn exhausted_api_budgets_and_invalid_endpoints_never_request() {
        let mut reader = PrDataReader::with_transport(NoRequest);
        reader.encoded = MAX_API_BYTES;
        assert!(
            reader
                .json::<serde_json::Value>("pulls/7", MAX_TREE_BYTES)
                .is_err()
        );
        let mut reader = PrDataReader::with_transport(NoRequest);
        reader.requests = MAX_REQUESTS;
        assert!(
            reader
                .json::<serde_json::Value>("pulls/7", MAX_TREE_BYTES)
                .is_err()
        );
        let mut reader = PrDataReader::with_transport(NoRequest);
        reader.deadline = Instant::now();
        assert!(
            reader
                .json::<serde_json::Value>("pulls/7", MAX_TREE_BYTES)
                .is_err()
        );
        for path in [
            "https://evil.invalid",
            "git/trees/HEAD",
            "pulls/0",
            "pulls/7?redirect=evil",
            "git/ref/heads/other",
        ] {
            assert!(
                PrDataReader::with_transport(NoRequest)
                    .json::<serde_json::Value>(path, MAX_TREE_BYTES)
                    .is_err()
            );
        }
    }
}
