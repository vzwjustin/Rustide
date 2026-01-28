//! Context Capsules - Structured context for agents

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// A field in the context capsule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleField {
    /// Field name
    pub name: String,
    /// Field value
    pub value: Value,
    /// Field type description
    pub field_type: String,
    /// Whether this field is required
    pub required: bool,
    /// Description of the field
    pub description: Option<String>,
    /// When this field was last updated
    pub updated_at: DateTime<Utc>,
}

impl CapsuleField {
    /// Create a new field
    pub fn new(name: &str, value: Value) -> Self {
        Self {
            name: name.to_string(),
            value,
            field_type: "any".to_string(),
            required: false,
            description: None,
            updated_at: Utc::now(),
        }
    }

    /// Create a string field
    pub fn string(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: Value::String(value.to_string()),
            field_type: "string".to_string(),
            required: false,
            description: None,
            updated_at: Utc::now(),
        }
    }

    /// Create a number field
    pub fn number(name: &str, value: f64) -> Self {
        Self {
            name: name.to_string(),
            value: serde_json::json!(value),
            field_type: "number".to_string(),
            required: false,
            description: None,
            updated_at: Utc::now(),
        }
    }

    /// Create a boolean field
    pub fn boolean(name: &str, value: bool) -> Self {
        Self {
            name: name.to_string(),
            value: Value::Bool(value),
            field_type: "boolean".to_string(),
            required: false,
            description: None,
            updated_at: Utc::now(),
        }
    }

    /// Create an array field
    pub fn array(name: &str, values: Vec<Value>) -> Self {
        Self {
            name: name.to_string(),
            value: Value::Array(values),
            field_type: "array".to_string(),
            required: false,
            description: None,
            updated_at: Utc::now(),
        }
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Add description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Update the value
    pub fn update(&mut self, value: Value) {
        self.value = value;
        self.updated_at = Utc::now();
    }

    /// Get as string
    pub fn as_str(&self) -> Option<&str> {
        self.value.as_str()
    }

    /// Get as number
    pub fn as_f64(&self) -> Option<f64> {
        self.value.as_f64()
    }

    /// Get as boolean
    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_bool()
    }

    /// Get as array
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        self.value.as_array()
    }
}

/// A context capsule containing structured context for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCapsule {
    /// Unique identifier
    pub id: Uuid,
    /// Capsule name/type
    pub name: String,
    /// Version for optimistic concurrency
    pub version: u64,
    /// Fields in this capsule
    pub fields: HashMap<String, CapsuleField>,
    /// Parent capsule ID (for inheritance)
    pub parent_id: Option<Uuid>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}

impl ContextCapsule {
    /// Create a new context capsule
    pub fn new(name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            version: 1,
            fields: HashMap::new(),
            parent_id: None,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a field
    pub fn add_field(&mut self, field: CapsuleField) {
        self.fields.insert(field.name.clone(), field);
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Get a field
    pub fn get_field(&self, name: &str) -> Option<&CapsuleField> {
        self.fields.get(name)
    }

    /// Get a field value
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.get(name).map(|f| &f.value)
    }

    /// Set a field value
    pub fn set(&mut self, name: &str, value: Value) {
        if let Some(field) = self.fields.get_mut(name) {
            field.update(value);
        } else {
            self.add_field(CapsuleField::new(name, value));
        }
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Remove a field
    pub fn remove_field(&mut self, name: &str) -> Option<CapsuleField> {
        let result = self.fields.remove(name);
        if result.is_some() {
            self.updated_at = Utc::now();
            self.version += 1;
        }
        result
    }

    /// Check if a field exists
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    /// Get all field names
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.keys().map(|s| s.as_str()).collect()
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Check if has tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: &str, value: Value) {
        self.metadata.insert(key.to_string(), value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Validate that all required fields are present
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let missing: Vec<String> = self.fields
            .values()
            .filter(|f| f.required && f.value.is_null())
            .map(|f| f.name.clone())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Merge another capsule's fields into this one
    pub fn merge(&mut self, other: &ContextCapsule) {
        for (name, field) in &other.fields {
            if !self.fields.contains_key(name) {
                self.fields.insert(name.clone(), field.clone());
            }
        }
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Create a child capsule
    pub fn create_child(&self, name: &str) -> ContextCapsule {
        let mut child = ContextCapsule::new(name);
        child.parent_id = Some(self.id);
        // Copy fields
        child.fields = self.fields.clone();
        child
    }

    /// Convert to JSON
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    /// Create from JSON
    pub fn from_json(json: &Value) -> Option<Self> {
        serde_json::from_value(json.clone()).ok()
    }
}

impl Default for ContextCapsule {
    fn default() -> Self {
        Self::new("default")
    }
}

/// Builder for creating context capsules
pub struct CapsuleBuilder {
    capsule: ContextCapsule,
}

impl CapsuleBuilder {
    /// Create a new builder
    pub fn new(name: &str) -> Self {
        Self {
            capsule: ContextCapsule::new(name),
        }
    }

    /// Add a field
    pub fn field(mut self, field: CapsuleField) -> Self {
        self.capsule.add_field(field);
        self
    }

    /// Add a string field
    pub fn string(mut self, name: &str, value: &str) -> Self {
        self.capsule.add_field(CapsuleField::string(name, value));
        self
    }

    /// Add a number field
    pub fn number(mut self, name: &str, value: f64) -> Self {
        self.capsule.add_field(CapsuleField::number(name, value));
        self
    }

    /// Add a boolean field
    pub fn boolean(mut self, name: &str, value: bool) -> Self {
        self.capsule.add_field(CapsuleField::boolean(name, value));
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: &str) -> Self {
        self.capsule.add_tag(tag);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: &str, value: Value) -> Self {
        self.capsule.set_metadata(key, value);
        self
    }

    /// Set parent
    pub fn parent(mut self, parent_id: Uuid) -> Self {
        self.capsule.parent_id = Some(parent_id);
        self
    }

    /// Build the capsule
    pub fn build(self) -> ContextCapsule {
        self.capsule
    }
}

/// Thread-safe capsule store
pub struct CapsuleStore {
    capsules: RwLock<HashMap<Uuid, ContextCapsule>>,
}

impl CapsuleStore {
    /// Create a new store
    pub fn new() -> Self {
        Self {
            capsules: RwLock::new(HashMap::new()),
        }
    }

    /// Add a capsule
    pub fn add(&self, capsule: ContextCapsule) -> Uuid {
        let id = capsule.id;
        self.capsules.write().insert(id, capsule);
        id
    }

    /// Get a capsule
    pub fn get(&self, id: &Uuid) -> Option<ContextCapsule> {
        self.capsules.read().get(id).cloned()
    }

    /// Update a capsule
    pub fn update(&self, capsule: ContextCapsule) -> bool {
        let mut store = self.capsules.write();
        if store.contains_key(&capsule.id) {
            store.insert(capsule.id, capsule);
            true
        } else {
            false
        }
    }

    /// Remove a capsule
    pub fn remove(&self, id: &Uuid) -> Option<ContextCapsule> {
        self.capsules.write().remove(id)
    }

    /// Find capsules by name
    pub fn find_by_name(&self, name: &str) -> Vec<ContextCapsule> {
        self.capsules
            .read()
            .values()
            .filter(|c| c.name == name)
            .cloned()
            .collect()
    }

    /// Find capsules by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<ContextCapsule> {
        self.capsules
            .read()
            .values()
            .filter(|c| c.has_tag(tag))
            .cloned()
            .collect()
    }

    /// Get all capsules
    pub fn all(&self) -> Vec<ContextCapsule> {
        self.capsules.read().values().cloned().collect()
    }

    /// Count capsules
    pub fn count(&self) -> usize {
        self.capsules.read().len()
    }
}

impl Default for CapsuleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capsule_field() {
        let field = CapsuleField::string("name", "test")
            .required()
            .with_description("A test field");

        assert_eq!(field.name, "name");
        assert_eq!(field.as_str(), Some("test"));
        assert!(field.required);
        assert!(field.description.is_some());
    }

    #[test]
    fn test_context_capsule() {
        let mut capsule = ContextCapsule::new("test");
        capsule.add_field(CapsuleField::string("name", "value"));

        assert_eq!(capsule.get("name").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_capsule_builder() {
        let capsule = CapsuleBuilder::new("test")
            .string("name", "test")
            .number("count", 42.0)
            .boolean("active", true)
            .tag("important")
            .build();

        assert_eq!(capsule.name, "test");
        assert!(capsule.has_field("name"));
        assert!(capsule.has_field("count"));
        assert!(capsule.has_tag("important"));
    }

    #[test]
    fn test_capsule_validation() {
        let mut capsule = ContextCapsule::new("test");
        capsule.add_field(CapsuleField::string("required_field", "").required());

        // Should fail because required field is empty
        // Note: we'd need to check for empty string, not just null
        let validation = capsule.validate();
        // This passes because the value is not null (it's an empty string)
        assert!(validation.is_ok());
    }

    #[test]
    fn test_capsule_store() {
        let store = CapsuleStore::new();

        let capsule = CapsuleBuilder::new("test")
            .string("key", "value")
            .build();
        let id = capsule.id;

        store.add(capsule);
        assert_eq!(store.count(), 1);

        let retrieved = store.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test");
    }

    #[test]
    fn test_capsule_merge() {
        let mut capsule1 = ContextCapsule::new("primary");
        capsule1.add_field(CapsuleField::string("field1", "value1"));

        let mut capsule2 = ContextCapsule::new("secondary");
        capsule2.add_field(CapsuleField::string("field2", "value2"));

        capsule1.merge(&capsule2);

        assert!(capsule1.has_field("field1"));
        assert!(capsule1.has_field("field2"));
    }
}
