use std::collections::HashMap;
use std::fs;
use std::path::Path;

use failure::Fallible;
use log::*;
use serde_derive::{Deserialize, Serialize};

use did::model::ContentId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaVersion {
    pub id: ContentId,
    pub author: String,
    pub name: String,
    pub version: u32,
    pub content: serde_json::Value,
}

pub struct ClaimSchemaRegistry {
    // TODO Implement Iterable<SchemaVersion> and remove pub from here
    pub schemas: HashMap<ContentId, SchemaVersion>,
}

impl ClaimSchemaRegistry {
    pub fn import_folder(path: &Path) -> Fallible<Self> {
        use std::io::ErrorKind;
        let mut root = ClaimSchemaRegistry { schemas: Default::default() };
        for entry in path.read_dir()? {
            // Iterator.next() might fail and then iteration should stop
            let entry = entry?.path();
            match fs::read_to_string(&entry) {
                Ok(content) => {
                    let schema_version_res = serde_json::from_str::<SchemaVersion>(&content);
                    match schema_version_res {
                        Ok(schema_version_borrow) => {
                            let schema_version = schema_version_borrow.to_owned();
                            root.schemas.insert(schema_version.id.clone(), schema_version);
                        }
                        Err(e) => {
                            warn!(
                                "Claim schema '{}' is not a schema: {}",
                                entry.to_string_lossy(),
                                e
                            );
                        }
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::InvalidInput => {
                    warn!(
                        "Claim schema '{}' contains invalid characters: {}",
                        entry.to_string_lossy(),
                        e
                    );
                }
                Err(ref e) if e.kind() == ErrorKind::InvalidData => {
                    debug!("Directory entry '{}' is not a file: {}", entry.to_string_lossy(), e);
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(root)
    }
}
