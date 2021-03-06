pub mod rate;
pub mod request;
pub mod version;

pub use crate::request::Content;
use crate::version::Version;

pub use anyhow::Result;
use chrono::{DateTime, Utc};
pub use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
pub use serde_hjson::value::ToJson;
use serde_json::json;
use std::collections::HashMap;
pub use tokio::prelude::*;

/// Deserializes mods from list at: https://github.com/Anuken/MindustryMods/blob/master/mods.json
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModSource {
    /// ex: `"What42Pizza/Mindustry-Production-Mod"`
    pub repo: String,

    /// ex: `"Mindustry-Production-Mod"`
    name: String,

    /// ex: `"[orange]What42Pizza"`
    author: String,

    /// ex: `"2020-03-18T16:35:29Z"`
    last_updated: String,

    /// ex: `25`
    stars: u32,

    /// ex: `"[white]This mod gives you [orange]iron[white]..."`
    description: String,
}

/// The `mod.json` file.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    name: Option<String>,
    description: Option<String>,
    author: Option<String>,
    version: Version,
    dependencies: Option<Vec<String>>,
    display_name: Option<String>,
    min_game_version: Option<String>,
    hidden: Option<bool>,
    main_script: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Assets {
    Content,
    Bundles,
    Sounds,
    Schematics,
    SpritesOverride,
    Sprites,
    Scripts,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Contents {
    Items,
    Blocks,
    Mechs,
    Liquids,
    Units,
    Zones,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Cache {
    name: String,
    stars: u32,
    date: DateTime<Utc>,
    sha: String,
    mod_info: ModInfo,
    readme: String,
    assets: Vec<Assets>,
    contents: Vec<Contents>,
}

/// Type to allow conversion of Hjson and Json value.
/// This is required because serde_hjson uses an older
/// version of serde. (serde 0.7)
pub struct Hjson(pub serde_hjson::Value);

impl From<Hjson> for serde_json::Value {
    fn from(value: Hjson) -> Self {
        let Hjson(value) = value;
        match value {
            serde_hjson::Value::Null => serde_json::Value::Null,
            serde_hjson::Value::Bool(x) => json!(x),
            serde_hjson::Value::I64(x) => json!(x),
            serde_hjson::Value::U64(x) => json!(x),
            serde_hjson::Value::F64(x) => json!(x),
            serde_hjson::Value::String(x) => json!(x),
            serde_hjson::Value::Array(x) => {
                let vector = x
                    .into_iter()
                    .map(|x| Hjson(x).into())
                    .collect::<Vec<serde_json::Value>>();
                json!(vector)
            }
            serde_hjson::Value::Object(x) => {
                let hash_map = x
                    .into_iter()
                    .map(|(k, v)| (k, Hjson(v).into()))
                    .collect::<HashMap<_, serde_json::Value>>();
                json!(hash_map)
            }
        }
    }
}
