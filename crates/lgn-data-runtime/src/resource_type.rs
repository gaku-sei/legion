use std::{collections::HashMap, fmt, sync::RwLock};

use crate::Resource;
use lgn_content_store::indexing::IndexKey;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use xxhash_rust::const_xxh3::xxh3_64 as const_xxh3;

/// Type identifier of resource or asset.
///
/// It is currently generated by hashing the name of a type, into a stable
/// 64-bits value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ResourceType(u64);

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:016x}", self.0))
    }
}
impl fmt::Debug for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ResourceType")
            .field(&format_args!("{:#016x}", self.0))
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct ResourceTypeEntry {
    pub(crate) name: &'static str,
    pub(crate) new_instance: fn() -> Box<dyn Resource>,
}

static RESOURCE_TYPE_REGISTRY: OnceCell<RwLock<HashMap<ResourceType, ResourceTypeEntry>>> =
    OnceCell::new();

impl ResourceType {
    /// Creates a new type id from series of bytes.
    ///
    /// It is recommended to use this method to define a public constant
    /// which can be used to identify a resource or asset.
    pub const fn new(v: &[u8]) -> Self {
        Self::from_raw(const_xxh3(v))
    }

    /// Creates a type id from a non-zero integer.
    pub const fn from_raw(v: u64) -> Self {
        let v = match std::num::NonZeroU64::new(v) {
            Some(v) => v,
            None => panic!(),
        };
        Self(v.get())
    }

    /// Return the available resource type that can be created
    pub fn get_resource_types() -> Vec<(ResourceType, &'static str)> {
        let name_mapping = RESOURCE_TYPE_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        name_mapping
            .read()
            .unwrap()
            .iter()
            .map(|(k, entry)| (*k, entry.name))
            .collect()
    }

    /// Return the name of the `ResourceType`
    pub fn as_pretty(self) -> &'static str {
        let name_mapping = RESOURCE_TYPE_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        if let Some(value) = name_mapping.read().unwrap().get(&self) {
            value.name
        } else {
            "unknown"
        }
    }

    /// Return a new instance of the type
    pub fn new_instance(self) -> Box<dyn Resource> {
        let name_mapping = RESOURCE_TYPE_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        let entry = name_mapping
            .read()
            .unwrap()
            .get(&self)
            .unwrap_or_else(|| panic!("Unregistered type {}", self))
            .clone();
        (entry.new_instance)()
    }

    pub(crate) fn register_type(id: ResourceType, entry: ResourceTypeEntry) {
        let name_mapping = RESOURCE_TYPE_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        name_mapping.write().unwrap().insert(id, entry);
    }
}

impl Serialize for ResourceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let bytes = self.0.to_be_bytes();
            let hex = hex::encode(bytes);
            serializer.serialize_str(&hex)
        } else {
            serializer.serialize_u64(self.0)
        }
    }
}

impl<'de> Deserialize<'de> for ResourceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let v = {
            if deserializer.is_human_readable() {
                let hex = String::deserialize(deserializer)?;
                let digits = hex::decode(hex).map_err(D::Error::custom)?;
                u64::from_be_bytes(digits.try_into().unwrap())
            } else {
                u64::deserialize(deserializer)?
            }
        };
        Ok(Self::from_raw(v))
    }
}

impl std::str::FromStr for ResourceType {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = u64::from_str_radix(s, 16)?;
        if v == 0 {
            Err("Z".parse::<u64>().expect_err("ParseIntError"))
        } else {
            Ok(Self::from_raw(v))
        }
    }
}

impl From<ResourceType> for IndexKey {
    fn from(kind: ResourceType) -> Self {
        kind.0.into()
    }
}

impl From<IndexKey> for ResourceType {
    fn from(key: IndexKey) -> Self {
        Self::from_raw(key.into())
    }
}