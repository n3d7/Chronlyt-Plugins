#![allow(dead_code)]
use crate::common;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chronlyt_registry::{
    RegistryResult,
    transport::{HttpResponse, HttpTransport},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Cursor, time::Duration};

pub const API: &str = "https://api.github.com/repos/n3d7/Chronlyt-Plugins/";
pub fn sha(c: char) -> String {
    c.to_string().repeat(40)
}
pub struct Mock {
    pub bodies: BTreeMap<String, Vec<u8>>,
    pub calls: Vec<String>,
}
impl HttpTransport for Mock {
    fn get(&mut self, url: &str, timeout: Duration) -> RegistryResult<HttpResponse> {
        assert!(url.starts_with(API));
        assert!(timeout <= Duration::from_secs(30));
        self.calls.push(url.into());
        let bytes = self.bodies.get(url).expect("unexpected endpoint").clone();
        Ok(HttpResponse {
            status: 200,
            location: None,
            content_length: Some(bytes.len() as u64),
            body: Box::new(Cursor::new(bytes)),
        })
    }
}
impl Mock {
    pub fn put(&mut self, path: &str, value: Value) {
        self.bodies
            .insert(format!("{API}{path}"), serde_json::to_vec(&value).unwrap());
    }
    pub fn change(&mut self, path: &str, f: impl FnOnce(&mut Value)) {
        let key = format!("{API}{path}");
        let mut value = serde_json::from_slice(&self.bodies[&key]).unwrap();
        f(&mut value);
        self.put(path, value);
    }
    pub fn blob(&mut self, id: char, bytes: &[u8]) {
        self.put(&format!("git/blobs/{}",sha(id)),json!({"sha":sha(id),"size":bytes.len(),"encoding":"base64","content":STANDARD.encode(bytes)}));
    }
    pub fn tree(&mut self, id: char, entries: Value) {
        self.put(
            &format!("git/trees/{}", sha(id)),
            json!({"sha":sha(id),"truncated":false,"tree":entries}),
        );
    }
}
pub fn entry(name: &str, id: char, kind: &str, mode: &str, size: Option<usize>) -> Value {
    json!({"path":name,"sha":sha(id),"type":kind,"mode":mode,"size":size,"url":"https://evil.invalid/ignored"})
}
pub fn mock() -> Mock {
    let mut mock = Mock {
        bodies: BTreeMap::new(),
        calls: vec![],
    };
    let metadata = common::metadata();
    let record = serde_json::to_vec(&common::record()).unwrap();
    let catalog = include_bytes!("../../../../catalog.json");
    mock.put("pulls/7",json!({"number":7,"state":"open","head":{"sha":sha('a'),"repo":{"clone_url":"file:///untrusted"}},"base":{"sha":sha('b'),"ref":"main","repo":{"full_name":"n3d7/Chronlyt-Plugins"}}}));
    mock.put(
        "git/ref/heads/main",
        json!({"ref":"refs/heads/main","object":{"type":"commit","sha":sha('b')}}),
    );
    mock.put(
        &format!("git/commits/{}", sha('a')),
        json!({"sha":sha('a'),"tree":{"sha":sha('c')}}),
    );
    mock.tree(
        'c',
        json!([
            entry("registry", 'd', "tree", "040000", None),
            entry("catalog.json", '2', "blob", "100644", Some(catalog.len())),
            entry("build.rs", '3', "blob", "100755", Some(99)),
            entry(".cargo", '3', "tree", "040000", None)
        ]),
    );
    mock.tree(
        'd',
        json!([
            entry("metadata.json", 'f', "blob", "100644", Some(metadata.len())),
            entry("plugins", 'e', "tree", "040000", None)
        ]),
    );
    mock.tree(
        'e',
        json!([entry(
            "example.notes.json",
            '1',
            "blob",
            "100644",
            Some(record.len())
        )]),
    );
    mock.blob('f', &metadata);
    mock.blob('1', &record);
    mock.blob('2', catalog);
    mock
}
