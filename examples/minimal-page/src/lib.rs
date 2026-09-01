include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use chronlyt_plugin_contracts::{PluginNodeV1, PluginViewV1};

struct MinimalCanary;

impl Guest for MinimalCanary {
    fn render_page(page_id: String, _state_json: String) -> Result<String, String> {
        render_page(&page_id)
    }

    fn handle_event(
        page_id: String,
        action_id: String,
        _input_json: String,
    ) -> Result<String, String> {
        handle_event(&page_id, &action_id)
    }
}

export!(MinimalCanary);

fn render_page(page_id: &str) -> Result<String, String> {
    if page_id == "main" {
        Ok(view("ready"))
    } else {
        Err("unknown page".into())
    }
}

fn handle_event(page_id: &str, action_id: &str) -> Result<String, String> {
    if page_id != "main" {
        return Err("unknown page".into());
    }
    if action_id != "ping" {
        return Err("unknown action".into());
    }
    Ok(view("pong"))
}

fn view(text: &str) -> String {
    let value = PluginViewV1 {
        schema_version: 1,
        root: PluginNodeV1::Page {
            title: Some("Compatibility canary".into()),
            children: vec![
                PluginNodeV1::Text {
                    text: text.into(),
                    tone: None,
                },
                PluginNodeV1::Button {
                    label: "Ping".into(),
                    action_id: "ping".into(),
                    variant: None,
                    disabled: false,
                },
            ],
        },
    };
    serde_json::to_string(&value).expect("static canary view must serialize")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_and_event_return_the_complete_valid_fixture_views() {
        for (actual, fixture) in [
            (
                render_page("main").unwrap(),
                include_bytes!("../../../fixtures/v1/canary/ready.json").as_slice(),
            ),
            (
                handle_event("main", "ping").unwrap(),
                include_bytes!("../../../fixtures/v1/canary/pong.json").as_slice(),
            ),
        ] {
            let actual = PluginViewV1::parse_and_validate(actual.as_bytes()).unwrap();
            let expected = PluginViewV1::parse_and_validate(fixture).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn unknown_page_and_action_fail_without_host_calls() {
        assert_eq!(render_page("other"), Err("unknown page".into()));
        assert_eq!(handle_event("other", "ping"), Err("unknown page".into()));
        assert_eq!(handle_event("main", "other"), Err("unknown action".into()));
    }
}
