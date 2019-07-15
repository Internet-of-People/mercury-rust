mod defaults;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use failure::Fallible;
use log::*;
use serde_derive::{Deserialize, Serialize};

use did::model::ContentId;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaVersion {
    id: ContentId,
    author: String,
    name: String,
    version: u32,
    content: serde_json::Value,
}

impl SchemaVersion {
    pub fn new(
        id: impl ToString,
        author: impl ToString,
        name: impl ToString,
        version: u32,
        content: serde_json::Value,
    ) -> Self {
        Self {
            id: id.to_string(),
            author: author.to_string(),
            name: name.to_string(),
            version,
            content,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn content(&self) -> &serde_json::Value {
        &self.content
    }
}

pub struct ClaimSchemaRegistry {
    schemas: HashMap<ContentId, SchemaVersion>,
}

impl ClaimSchemaRegistry {
    pub fn iter(&self) -> impl Iterator<Item = &SchemaVersion> {
        self.schemas.values()
    }

    pub fn populate_folder(path: &Path) -> Fallible<()> {
        for schema in defaults::get() {
            let file_name =
                format!("{}_{}_{}.schema.json", schema.author, schema.name, schema.version);
            let file = std::fs::File::create(&path.join(file_name))?;
            serde_json::to_writer(file, &schema)?;
        }
        Ok(())
    }

    pub fn import_folder(path: &Path) -> Fallible<Self> {
        let mut root = ClaimSchemaRegistry { schemas: Default::default() };
        for entry in path.read_dir()? {
            // Iterator.next() might fail and then iteration should stop
            let entry = entry?.path();
            root.import_file(&entry)?;
        }
        Ok(root)
    }

    fn import_file(&mut self, path: &Path) -> Fallible<()> {
        use std::io::ErrorKind;
        match fs::read_to_string(path) {
            Ok(content) => self.import_content(&content).or_else(|e| {
                warn!("Claim schema '{}' is not a schema: {}", path.to_string_lossy(), e);
                Ok(())
            }),
            Err(ref e) if e.kind() == ErrorKind::InvalidInput => {
                warn!(
                    "Claim schema '{}' contains invalid characters: {}",
                    path.to_string_lossy(),
                    e
                );
                Ok(())
            }
            Err(ref e) if e.kind() == ErrorKind::InvalidData => {
                debug!("Directory entry '{}' is not a file: {}", path.to_string_lossy(), e);
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    fn import_content(&mut self, content: &str) -> Fallible<()> {
        let schema_version = serde_json::from_str::<SchemaVersion>(&content)?;
        self.schemas.insert(schema_version.id.clone(), schema_version);
        Ok(())
    }
}
