//! Credential-free artifact transport. GitHub API credentials must use a
//! separate client and must never be passed to this transport or its redirects.
use crate::{ArtifactFetcher, FetchBudget, RegistryError, RegistryResult};
use chronlyt_plugin_contracts::{
    limits::MAX_PACKAGE_BYTES, validate_artifact_url, validate_redirect_url,
};
use reqwest::{
    blocking::{Client, Response},
    redirect::Policy,
};
use std::{
    collections::BTreeSet,
    io::Read,
    time::{Duration, Instant},
};
use url::Url;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_REDIRECTS: usize = 3;
const MAX_LOCATION_BYTES: usize = 8 * 1024;

pub struct HttpResponse {
    pub status: u16,
    pub location: Option<String>,
    pub content_length: Option<u64>,
    pub body: Box<dyn Read + Send>,
}

/// Test seam for HTTP bytes only; production always uses the fixed TLS client.
pub trait HttpTransport {
    fn get(&mut self, url: &str, timeout: Duration) -> RegistryResult<HttpResponse>;
}

pub struct ReqwestTransport {
    client: Client,
}

pub(crate) fn credential_free_client() -> RegistryResult<Client> {
    Client::builder()
        .https_only(true)
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .redirect(Policy::none())
        .referer(false)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("Chronlyt-Registry/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| RegistryError::Transport)
}

impl HttpTransport for ReqwestTransport {
    fn get(&mut self, url: &str, timeout: Duration) -> RegistryResult<HttpResponse> {
        let response = self
            .client
            .get(url)
            .timeout(timeout)
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .send()
            .map_err(|_| RegistryError::Transport)?;
        response_parts(response)
    }
}

pub(crate) fn response_parts(response: Response) -> RegistryResult<HttpResponse> {
    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .map(|value| {
            if value.len() > MAX_LOCATION_BYTES {
                return Err(RegistryError::Limit("redirect location"));
            }
            value
                .to_str()
                .map(str::to_owned)
                .map_err(|_| RegistryError::Transport)
        })
        .transpose()?;
    let content_length = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or(RegistryError::Transport)
        })
        .transpose()?;
    Ok(HttpResponse {
        status: response.status().as_u16(),
        location,
        content_length,
        body: Box::new(response),
    })
}

pub struct ArtifactClient<T = ReqwestTransport> {
    transport: T,
}

impl ArtifactClient<ReqwestTransport> {
    pub fn new() -> RegistryResult<Self> {
        Ok(Self {
            transport: ReqwestTransport {
                client: credential_free_client()?,
            },
        })
    }
}

impl<T: HttpTransport> ArtifactClient<T> {
    pub fn with_transport(transport: T) -> Self {
        Self { transport }
    }
}

impl<T: HttpTransport> ArtifactFetcher for ArtifactClient<T> {
    fn fetch(&mut self, url: &str, budget: FetchBudget) -> RegistryResult<Vec<u8>> {
        validate_artifact_url(url)?;
        let deadline = budget.deadline.min(Instant::now() + REQUEST_TIMEOUT);
        let max_bytes = budget.max_bytes.min(MAX_PACKAGE_BYTES);
        if max_bytes == 0 {
            return Err(RegistryError::Limit("artifact bytes"));
        }
        let mut url = Url::parse(url).map_err(|_| RegistryError::Transport)?;
        let mut visited = BTreeSet::new();
        for redirects in 0..=MAX_REDIRECTS {
            if !visited.insert(url.as_str().to_owned()) {
                return Err(RegistryError::Transport);
            }
            let timeout = deadline
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())
                .ok_or(RegistryError::Limit("artifact deadline"))?;
            // Passing the remaining total deadline into reqwest also bounds a
            // hanging response body, not just each individual blocking read.
            let response = self.transport.get(url.as_str(), timeout)?;
            if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
                if redirects == MAX_REDIRECTS {
                    return Err(RegistryError::Limit("artifact redirects"));
                }
                let location = response
                    .location
                    .as_deref()
                    .ok_or(RegistryError::Transport)?;
                if location.len() > MAX_LOCATION_BYTES {
                    return Err(RegistryError::Limit("redirect location"));
                }
                let next = url.join(location).map_err(|_| RegistryError::Transport)?;
                validate_redirect_url(next.as_str())?;
                url = next;
                continue;
            }
            if response.status != 200 {
                return Err(RegistryError::Transport);
            }
            return read_bounded_body(response, max_bytes, deadline);
        }
        Err(RegistryError::Transport)
    }
}

pub(crate) fn read_bounded_body(
    mut response: HttpResponse,
    max_bytes: usize,
    deadline: Instant,
) -> RegistryResult<Vec<u8>> {
    if response
        .content_length
        .is_some_and(|size| size > max_bytes as u64)
    {
        return Err(RegistryError::Limit("response bytes"));
    }
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if Instant::now() >= deadline {
            return Err(RegistryError::Limit("response deadline"));
        }
        let capacity = max_bytes
            .saturating_sub(bytes.len())
            .saturating_add(1)
            .min(chunk.len());
        let count = response
            .body
            .read(&mut chunk[..capacity])
            .map_err(|_| RegistryError::Transport)?;
        if Instant::now() >= deadline {
            return Err(RegistryError::Limit("response deadline"));
        }
        if count == 0 {
            break;
        }
        if count > max_bytes.saturating_sub(bytes.len()) {
            return Err(RegistryError::Limit("response bytes"));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    if response
        .content_length
        .is_some_and(|size| size != bytes.len() as u64)
    {
        return Err(RegistryError::Transport);
    }
    Ok(bytes)
}
