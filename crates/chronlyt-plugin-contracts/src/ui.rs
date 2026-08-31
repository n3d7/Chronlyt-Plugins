use serde::{Deserialize, Serialize};

use crate::{ContractError, ContractResult, limits::*, manifest::validate_identifier};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginToneV1 {
    Default,
    Muted,
    Accent,
    Success,
    Warning,
    Danger,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginSpacingV1 {
    Small,
    Medium,
    Large,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginButtonVariantV1 {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginListItemV1 {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
    pub title: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
    pub description: Option<String>,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginSelectOptionV1 {
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
    pub value: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
    pub label: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PluginNodeV1 {
    Page {
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        title: Option<String>,
        #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_UI_CHILDREN)))]
        children: Vec<PluginNodeV1>,
    },
    Stack {
        #[serde(default)]
        spacing: Option<PluginSpacingV1>,
        #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_UI_CHILDREN)))]
        children: Vec<PluginNodeV1>,
    },
    Row {
        #[serde(default)]
        spacing: Option<PluginSpacingV1>,
        #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_UI_CHILDREN)))]
        children: Vec<PluginNodeV1>,
    },
    Card {
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        title: Option<String>,
        #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_UI_CHILDREN)))]
        children: Vec<PluginNodeV1>,
    },
    Divider,
    Spacer {
        size: PluginSpacingV1,
    },
    Heading {
        #[cfg_attr(feature = "schema", schemars(extend("minimum" = 1, "maximum" = 3)))]
        level: u8,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        text: String,
    },
    Text {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        text: String,
        #[serde(default)]
        tone: Option<PluginToneV1>,
    },
    Badge {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        text: String,
        #[serde(default)]
        tone: Option<PluginToneV1>,
    },
    List {
        #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_UI_LIST_ITEMS)))]
        items: Vec<PluginListItemV1>,
    },
    Button {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        label: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_ACTION_ID_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        action_id: String,
        #[serde(default)]
        variant: Option<PluginButtonVariantV1>,
        #[serde(default)]
        disabled: bool,
    },
    TextInput {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_IDENTIFIER_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        field_id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        label: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        value: String,
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        placeholder: Option<String>,
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_ACTION_ID_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        action_id: Option<String>,
    },
    Select {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_IDENTIFIER_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        field_id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        label: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        value: String,
        #[cfg_attr(feature = "schema", schemars(extend("maxItems" = MAX_UI_LIST_ITEMS)))]
        options: Vec<PluginSelectOptionV1>,
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_ACTION_ID_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        action_id: Option<String>,
    },
    Checkbox {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_IDENTIFIER_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        field_id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        label: String,
        checked: bool,
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_ACTION_ID_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        action_id: Option<String>,
    },
    Toggle {
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_IDENTIFIER_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        field_id: String,
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_STRING_BYTES)))]
        label: String,
        checked: bool,
        #[serde(default)]
        #[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_UI_ACTION_ID_BYTES, "x-chronlyt-min-utf8-bytes" = 1)))]
        action_id: Option<String>,
    },
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(extend("x-chronlyt-max-utf8-bytes" = MAX_HOST_OUTPUT_BYTES, "x-chronlyt-max-depth" = MAX_UI_DEPTH, "x-chronlyt-max-nodes" = MAX_UI_NODES, "x-chronlyt-max-inputs" = MAX_UI_INPUTS)))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginViewV1 {
    #[cfg_attr(feature = "schema", schemars(extend("const" = 1)))]
    pub schema_version: u16,
    pub root: PluginNodeV1,
}

impl PluginViewV1 {
    pub fn parse_and_validate(bytes: &[u8]) -> ContractResult<Self> {
        if bytes.len() > MAX_HOST_OUTPUT_BYTES {
            return Err(ContractError::ResourceLimit("declarative view bytes"));
        }
        let view: Self = serde_json::from_slice(bytes)?;
        view.validate()?;
        Ok(view)
    }

    fn validate(&self) -> ContractResult<()> {
        if self.schema_version != 1 {
            return Err(ContractError::UnsupportedVersion);
        }
        let mut stack = vec![(&self.root, 1_usize)];
        let mut nodes = 0_usize;
        let mut inputs = 0_usize;
        while let Some((node, depth)) = stack.pop() {
            nodes += 1;
            if nodes > MAX_UI_NODES {
                return Err(ContractError::ResourceLimit("declarative UI nodes"));
            }
            if depth > MAX_UI_DEPTH {
                return Err(ContractError::ResourceLimit("declarative UI depth"));
            }
            match node {
                PluginNodeV1::Page { title, children } | PluginNodeV1::Card { title, children } => {
                    validate_optional_string(title)?;
                    push_children(&mut stack, children, depth)?;
                }
                PluginNodeV1::Stack { children, .. } | PluginNodeV1::Row { children, .. } => {
                    push_children(&mut stack, children, depth)?;
                }
                PluginNodeV1::Divider | PluginNodeV1::Spacer { .. } => {}
                PluginNodeV1::Heading { level, text } => {
                    if !(1..=3).contains(level) {
                        return Err(ContractError::InvalidData("heading level"));
                    }
                    validate_string(text)?;
                }
                PluginNodeV1::Text { text, .. } | PluginNodeV1::Badge { text, .. } => {
                    validate_string(text)?;
                }
                PluginNodeV1::List { items } => {
                    if items.len() > MAX_UI_LIST_ITEMS {
                        return Err(ContractError::ResourceLimit("declarative list items"));
                    }
                    for item in items {
                        validate_string(&item.title)?;
                        validate_optional_string(&item.description)?;
                    }
                }
                PluginNodeV1::Button {
                    label, action_id, ..
                } => {
                    validate_string(label)?;
                    validate_action_id(action_id)?;
                }
                PluginNodeV1::TextInput {
                    field_id,
                    label,
                    value,
                    placeholder,
                    action_id,
                } => {
                    inputs += 1;
                    validate_control(
                        field_id,
                        label,
                        value,
                        placeholder.as_ref(),
                        action_id.as_ref(),
                    )?;
                }
                PluginNodeV1::Select {
                    field_id,
                    label,
                    value,
                    options,
                    action_id,
                } => {
                    inputs += 1;
                    validate_control(field_id, label, value, None, action_id.as_ref())?;
                    if options.len() > MAX_UI_LIST_ITEMS {
                        return Err(ContractError::ResourceLimit("select options"));
                    }
                    for option in options {
                        validate_string(&option.value)?;
                        validate_string(&option.label)?;
                    }
                }
                PluginNodeV1::Checkbox {
                    field_id,
                    label,
                    action_id,
                    ..
                }
                | PluginNodeV1::Toggle {
                    field_id,
                    label,
                    action_id,
                    ..
                } => {
                    inputs += 1;
                    validate_control(field_id, label, "", None, action_id.as_ref())?;
                }
            }
            if inputs > MAX_UI_INPUTS {
                return Err(ContractError::ResourceLimit("declarative input fields"));
            }
        }
        Ok(())
    }
}

fn push_children<'a>(
    stack: &mut Vec<(&'a PluginNodeV1, usize)>,
    children: &'a [PluginNodeV1],
    depth: usize,
) -> ContractResult<()> {
    if children.len() > MAX_UI_CHILDREN {
        return Err(ContractError::ResourceLimit("declarative children"));
    }
    stack.extend(children.iter().rev().map(|child| (child, depth + 1)));
    Ok(())
}

fn validate_control(
    field_id: &str,
    label: &str,
    value: &str,
    placeholder: Option<&String>,
    action_id: Option<&String>,
) -> ContractResult<()> {
    validate_identifier(field_id, "field id")?;
    validate_string(label)?;
    validate_string(value)?;
    if let Some(placeholder) = placeholder {
        validate_string(placeholder)?;
    }
    if let Some(action_id) = action_id {
        validate_action_id(action_id)?;
    }
    Ok(())
}

fn validate_action_id(value: &str) -> ContractResult<()> {
    if !value.is_empty()
        && value.len() <= MAX_UI_ACTION_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        Ok(())
    } else {
        Err(ContractError::InvalidData("action id"))
    }
}

fn validate_string(value: &str) -> ContractResult<()> {
    if value.len() <= MAX_UI_STRING_BYTES {
        Ok(())
    } else {
        Err(ContractError::ResourceLimit("declarative string"))
    }
}

fn validate_optional_string(value: &Option<String>) -> ContractResult<()> {
    if let Some(value) = value {
        validate_string(value)?;
    }
    Ok(())
}
