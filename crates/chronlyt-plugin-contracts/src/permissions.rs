use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{ContractError, ContractResult, limits::MAX_PERMISSIONS};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    #[serde(rename = "storage.read")]
    StorageRead,
    #[serde(rename = "storage.write")]
    StorageWrite,
    #[serde(rename = "timeline.read")]
    TimelineRead,
    #[serde(rename = "timeline.write")]
    TimelineWrite,
    #[serde(rename = "notifications.show")]
    NotificationsShow,
}

impl Capability {
    pub fn from_id(value: &str) -> ContractResult<Self> {
        match value {
            "storage.read" => Ok(Self::StorageRead),
            "storage.write" => Ok(Self::StorageWrite),
            "timeline.read" => Ok(Self::TimelineRead),
            "timeline.write" => Ok(Self::TimelineWrite),
            "notifications.show" => Ok(Self::NotificationsShow),
            _ => Err(ContractError::InvalidData("permission id")),
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::StorageRead => "storage.read",
            Self::StorageWrite => "storage.write",
            Self::TimelineRead => "timeline.read",
            Self::TimelineWrite => "timeline.write",
            Self::NotificationsShow => "notifications.show",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::StorageRead => "Read this plugin's private local storage",
            Self::StorageWrite => "Change this plugin's private local storage",
            Self::TimelineRead => "Read a bounded selection of Timeline entries",
            Self::TimelineWrite => "Create or change this plugin's Timeline entries",
            Self::NotificationsShow => "Show bounded system notifications",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilitySet(BTreeSet<Capability>);

impl CapabilitySet {
    pub fn from_slice(values: &[Capability]) -> ContractResult<Self> {
        if values.len() > MAX_PERMISSIONS {
            return Err(ContractError::ResourceLimit("permission count"));
        }
        let set = values.iter().copied().collect::<BTreeSet<_>>();
        if set.len() != values.len() {
            return Err(ContractError::InvalidData("duplicate permission"));
        }
        Ok(Self(set))
    }

    pub fn contains(&self, capability: Capability) -> bool {
        self.0.contains(&capability)
    }

    pub fn iter(&self) -> impl Iterator<Item = Capability> + '_ {
        self.0.iter().copied()
    }

    pub fn delta(&self, requested: &Self) -> PermissionDelta {
        PermissionDelta {
            retained: self.0.intersection(&requested.0).copied().collect(),
            added: requested.0.difference(&self.0).copied().collect(),
            removed: self.0.difference(&requested.0).copied().collect(),
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionDelta {
    pub retained: BTreeSet<Capability>,
    pub added: BTreeSet<Capability>,
    pub removed: BTreeSet<Capability>,
}
