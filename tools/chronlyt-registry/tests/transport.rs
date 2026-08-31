use chronlyt_registry::transport::{ArtifactClient, HttpResponse, HttpTransport};
use chronlyt_registry::{ArtifactFetcher, FetchBudget, RegistryResult};
use std::{
    collections::VecDeque,
    io::{self, Cursor, Read},
    time::{Duration, Instant},
};

const URL: &str = "https://github.com/example/notes/releases/download/v1/a.chronlyt-plugin";
struct Mock {
    responses: VecDeque<HttpResponse>,
    urls: Vec<String>,
}
impl HttpTransport for Mock {
    fn get(&mut self, url: &str, timeout: Duration) -> RegistryResult<HttpResponse> {
        assert!(timeout <= Duration::from_secs(30));
        assert!(!timeout.is_zero());
        self.urls.push(url.into());
        Ok(self.responses.pop_front().expect("unexpected request"))
    }
}
fn response(status: u16, location: Option<&str>, length: Option<u64>, body: &[u8]) -> HttpResponse {
    HttpResponse {
        status,
        location: location.map(str::to_owned),
        content_length: length,
        body: Box::new(Cursor::new(body.to_vec())),
    }
}
fn client(responses: Vec<HttpResponse>) -> ArtifactClient<Mock> {
    ArtifactClient::with_transport(Mock {
        responses: responses.into(),
        urls: vec![],
    })
}
fn budget(bytes: usize) -> FetchBudget {
    FetchBudget {
        max_bytes: bytes,
        deadline: Instant::now() + Duration::from_secs(30),
    }
}

#[test]
fn initial_url_policy_is_checked_before_transport() {
    for url in [
        "http://github.com/a/b",
        "file:///tmp/a",
        "https://evil.invalid/a",
        "https://user:password@github.com/a",
        "https://github.com:444/a",
        "https://github.com/a?secret=x",
        "https://github.com/a#x",
    ] {
        assert!(client(vec![]).fetch(url, budget(100)).is_err());
    }
}

#[test]
fn manual_redirects_are_allowlisted_and_bounded() {
    let signed = "https://objects.githubusercontent.com/a?signature=test";
    let mut fetcher = client(vec![
        response(302, Some(signed), None, b""),
        response(200, None, Some(2), b"ok"),
    ]);
    assert_eq!(fetcher.fetch(URL, budget(100)).unwrap(), b"ok");
    for location in [
        "http://github.com/a",
        "https://evil.invalid/a",
        "https://objects.githubusercontent.com:444/a",
        "https://raw.githubusercontent.com/a?token=x",
        "https://user@objects.githubusercontent.com/a",
        "https://objects.githubusercontent.com/a#x",
        URL,
    ] {
        assert!(
            client(vec![response(302, Some(location), None, b"")])
                .fetch(URL, budget(100))
                .is_err()
        );
    }
    let mut fetcher = client(
        (1..=4)
            .map(|i| {
                response(
                    302,
                    Some(&format!("https://github.com/redirect/{i}")),
                    None,
                    b"",
                )
            })
            .collect(),
    );
    assert!(fetcher.fetch(URL, budget(100)).is_err());
}

#[test]
fn actual_body_and_advertised_sizes_must_agree_and_fit_budget() {
    assert_eq!(
        client(vec![response(200, None, None, b"ok")])
            .fetch(URL, budget(2))
            .unwrap(),
        b"ok"
    );
    for (length, body) in [
        (Some(3), b"ok".as_slice()),
        (Some(1), b"ok"),
        (None, b"too big"),
    ] {
        assert!(
            client(vec![response(200, None, length, body)])
                .fetch(URL, budget(2))
                .is_err()
        );
    }
    assert!(
        client(vec![response(404, None, None, b"not found")])
            .fetch(URL, budget(100))
            .is_err()
    );
}

struct TimedOut;
impl Read for TimedOut {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::TimedOut.into())
    }
}
#[test]
fn deadlines_and_body_timeouts_fail_without_leaking_url_queries() {
    let expired = FetchBudget {
        max_bytes: 100,
        deadline: Instant::now(),
    };
    assert!(client(vec![]).fetch(URL, expired).is_err());
    let mut fetcher = client(vec![HttpResponse {
        status: 200,
        location: None,
        content_length: None,
        body: Box::new(TimedOut),
    }]);
    let error = fetcher.fetch(URL, budget(100)).unwrap_err();
    assert!(!format!("{error:?}").contains("github.com"));
}
