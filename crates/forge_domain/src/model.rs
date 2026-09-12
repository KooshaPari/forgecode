use derive_more::derive::Display;
use derive_setters::Setters;
use fake::Dummy;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

/// Represents input modalities that a model can accept
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, JsonSchema, Dummy,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum InputModality {
    /// Text input (all models support this)
    Text,
    /// Image input (vision-capable models)
    Image,
}

/// Default input modalities when not specified (text-only)
fn default_input_modalities() -> Vec<InputModality> {
    vec![InputModality::Text]
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Setters, JsonSchema, Dummy)]
#[setters(strip_option)]
pub struct Model {
    pub id: ModelId,
    pub name: Option<String>,
    pub description: Option<String>,
    pub context_length: Option<u64>,
    // TODO: add provider information to the model
    pub tools_supported: Option<bool>,
    /// Whether the model supports parallel tool calls
    pub supports_parallel_tool_calls: Option<bool>,
    /// Whether the model supports reasoning
    pub supports_reasoning: Option<bool>,
    /// Input modalities supported by the model (defaults to text-only)
    #[serde(default = "default_input_modalities")]
    pub input_modalities: Vec<InputModality>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub tool_supported: bool,
}

impl Parameters {
    pub fn new(tool_supported: bool) -> Self {
        Self { tool_supported }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Hash, Eq, Display, JsonSchema, Dummy)]
#[serde(transparent)]
pub struct ModelId(String);

impl ModelId {
    pub fn new<T: Into<String>>(id: T) -> Self {
        Self(id.into())
    }
}

impl Model {
    /// Creates a new `Model` with the given id and default values for all other
    /// fields.
    pub fn new(id: impl Into<ModelId>) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
            context_length: None,
            tools_supported: None,
            supports_parallel_tool_calls: None,
            supports_reasoning: None,
            input_modalities: default_input_modalities(),
        }
    }

    /// Merges live model ids with curated metadata.
    ///
    /// Every live model id produces an entry; curated entries with a matching
    /// id overlay their metadata (name, context length, tool/reasoning
    /// support, modalities). Curated entries not present in the live list are
    /// appended so metadata-only models (e.g. behind beta flags) remain
    /// selectable.
    pub fn merge_live(live_ids: Vec<String>, curated: Vec<Model>) -> Vec<Self> {
        let mut merged: Vec<Self> = live_ids
            .into_iter()
            .map(|id| match curated.iter().find(|m| m.id.as_str() == id) {
                Some(curated_model) => {
                    let mut model = Self::new(id);
                    model.name = curated_model.name.clone();
                    model.description = curated_model.description.clone();
                    model.context_length = curated_model.context_length;
                    model.tools_supported = curated_model.tools_supported;
                    model.supports_parallel_tool_calls = curated_model.supports_parallel_tool_calls;
                    model.supports_reasoning = curated_model.supports_reasoning;
                    model.input_modalities = curated_model.input_modalities.clone();
                    model
                }
                None => Self::new(id),
            })
            .collect();

        for curated_model in curated {
            if !merged.iter().any(|m| m.id == curated_model.id) {
                merged.push(curated_model);
            }
        }
        merged
    }
}

impl From<String> for ModelId {
    fn from(value: String) -> Self {
        ModelId(value)
    }
}

impl From<&str> for ModelId {
    fn from(value: &str) -> Self {
        ModelId(value.to_string())
    }
}

impl ModelId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for ModelId {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ModelId(s.to_string()))
    }
}

#[cfg(test)]
mod merge_live_tests {
    use super::*;

    #[test]
    fn merge_live_emits_one_entry_per_live_id_with_default_metadata() {
        let merged = Model::merge_live(vec!["a".to_string(), "b".to_string()], vec![]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id.as_str(), "a");
        assert_eq!(merged[1].id.as_str(), "b");
        assert_eq!(merged[0].context_length, None);
        assert_eq!(merged[0].tools_supported, None);
        assert_eq!(merged[0].input_modalities, vec![InputModality::Text]);
    }

    #[test]
    fn merge_live_overlays_curated_metadata_onto_matching_live_id() {
        let curated = Model::new("a")
            .name("Alpha".to_string())
            .context_length(131072)
            .tools_supported(true)
            .supports_reasoning(true)
            .input_modalities(vec![InputModality::Text, InputModality::Image]);

        let merged = Model::merge_live(vec!["a".to_string()], vec![curated]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id.as_str(), "a");
        assert_eq!(merged[0].name.as_deref(), Some("Alpha"));
        assert_eq!(merged[0].context_length, Some(131072));
        assert_eq!(merged[0].tools_supported, Some(true));
        assert_eq!(merged[0].supports_reasoning, Some(true));
        assert_eq!(
            merged[0].input_modalities,
            vec![InputModality::Text, InputModality::Image]
        );
    }

    #[test]
    fn merge_live_appends_curated_entries_missing_from_live_list() {
        let curated_beta = Model::new("beta-only")
            .name("Beta".to_string())
            .context_length(8192);

        let merged = Model::merge_live(vec!["a".to_string()], vec![curated_beta]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id.as_str(), "a");
        assert_eq!(merged[1].id.as_str(), "beta-only");
        assert_eq!(merged[1].context_length, Some(8192));
    }

    #[test]
    fn merge_live_deduplicates_curated_entries_that_already_match_live_ids() {
        let curated = Model::new("a").name("Alpha".to_string());

        let merged = Model::merge_live(vec!["a".to_string()], vec![curated]);

        // "a" appears once in the merged result, not twice.
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id.as_str(), "a");
    }
}
