use crate::{ContractError, ContractResult};
use url::Url;

pub fn validate_repository_url(value: &str) -> ContractResult<()> {
    validate_https_url(value, &["github.com"], false)
}

pub fn validate_artifact_url(value: &str) -> ContractResult<()> {
    validate_https_url(
        value,
        &["github.com", "objects.githubusercontent.com"],
        false,
    )
}

pub fn validate_redirect_url(value: &str) -> ContractResult<()> {
    let parsed = Url::parse(value)?;
    let query_allowed = parsed.host_str() == Some("objects.githubusercontent.com");
    validate_https_url(
        value,
        &[
            "raw.githubusercontent.com",
            "github.com",
            "objects.githubusercontent.com",
        ],
        query_allowed,
    )
}

fn validate_https_url(
    value: &str,
    allowed_hosts: &[&str],
    allow_query: bool,
) -> ContractResult<()> {
    let url = Url::parse(value)?;
    let valid = url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| allowed_hosts.contains(&host))
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.fragment().is_none()
        && (allow_query || url.query().is_none());
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidData("URL allowlist"))
    }
}
