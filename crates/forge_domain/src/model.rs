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

    /// Merges live (server) models with curated metadata as fallback.
    ///
    /// Every live model produces an entry using the server-provided metadata.
    /// Curated entries fill in fields that the server left `None`, allowing
    /// curated data to act as a fallback without overriding live values.
    /// Curated entries not present in the live list are appended so
    /// metadata-only models (e.g. behind beta flags) remain selectable.
    pub fn merge_live(live: Vec<Model>, curated: Vec<Model>) -> Vec<Self> {
        let mut merged: Vec<Self> = Vec::with_capacity(live.len());
        for server_model in live {
            // First-seen wins on duplicate live entries
            if merged.iter().any(|m| m.id == server_model.id) {
                continue;
            }
            let mut model = match curated.iter().find(|m| m.id == server_model.id) {
                Some(curated_model) => {
                    let mut model = server_model;
                    // Server metadata wins; curated fills in None gaps
                    if model.name.is_none() {
                        model.name = curated_model.name.clone();
                    }
                    if model.description.is_none() {
                        model.description = curated_model.description.clone();
                    }
                    if model.context_length.is_none() {
                        model.context_length = curated_model.context_length;
                    }
                    if model.tools_supported.is_none() {
                        model.tools_supported = curated_model.tools_supported;
                    }
                    if model.supports_parallel_tool_calls.is_none() {
                        model.supports_parallel_tool_calls =
                            curated_model.supports_parallel_tool_calls;
                    }
                    if model.supports_reasoning.is_none() {
                        model.supports_reasoning = curated_model.supports_reasoning;
                    }
                    if model.input_modalities == vec![InputModality::Text]
                        && curated_model.input_modalities != vec![InputModality::Text]
                    {
                        model.input_modalities = curated_model.input_modalities.clone();
                    }
                    model
                }
                None => server_model,
            };
            merged.push(model);
        }

        // Append curated-only models not present in the live list
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
    fn merge_live_uses_server_metadata_when_available() {
        let live = vec![
            Model::new("a")
                .name("Alpha Live".to_string())
                .context_length(32768)
                .tools_supported(true),
            Model::new("b")
                .name("Beta Live".to_string())
                .context_length(16384),
        ];

        let merged = Model::merge_live(live, vec![]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id.as_str(), "a");
        assert_eq!(merged[0].name.as_deref(), Some("Alpha Live"));
        assert_eq!(merged[0].context_length, Some(32768));
        assert_eq!(merged[1].id.as_str(), "b");
        assert_eq!(merged[1].name.as_deref(), Some("Beta Live"));
    }

    #[test]
    fn merge_live_curated_fills_none_gaps() {
        let live = vec![Model::new("a").context_length(8192)];
        let curated = vec![Model::new("a")
            .name("Alpha Curated".to_string())
            .context_length(131072)
            .tools_supported(true)
            .supports_reasoning(true)
            .input_modalities(vec![InputModality::Text, InputModality::Image])];

        let merged = Model::merge_live(live, curated);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id.as_str(), "a");
        // Server context_length wins
        assert_eq!(merged[0].context_length, Some(8192));
        // Curated fills in name, tools, reasoning, modalities
        assert_eq!(merged[0].name.as_deref(), Some("Alpha Curated"));
        assert_eq!(merged[0].tools_supported, Some(true));
        assert_eq!(merged[0].supports_reasoning, Some(true));
        assert_eq!(
            merged[0].input_modalities,
            vec![InputModality::Text, InputModality::Image]
        );
    }

    #[test]
    fn merge_live_server_wins_over_curated() {
        let live = vec![Model::new("a")
            .name("Alpha Server".to_string())
            .context_length(32768)];
        let curated = vec![Model::new("a")
            .name("Alpha Curated".to_string())
            .context_length(131072)];

        let merged = Model::merge_live(live, curated);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name.as_deref(), Some("Alpha Server"));
        assert_eq!(merged[0].context_length, Some(32768));
    }

    #[test]
    fn merge_live_appends_curated_entries_missing_from_live_list() {
        let live = vec![Model::new("a").name("Alpha".to_string())];
        let curated = vec![Model::new("beta-only")
            .name("Beta".to_string())
            .context_length(8192)];

        let merged = Model::merge_live(live, curated);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id.as_str(), "a");
        assert_eq!(merged[1].id.as_str(), "beta-only");
        assert_eq!(merged[1].context_length, Some(8192));
    }

    #[test]
    fn merge_live_deduplicates_live_entries() {
        let live = vec![
            Model::new("a").name("Alpha First".to_string()),
            Model::new("a").name("Alpha Second".to_string()),
        ];

        let merged = Model::merge_live(live, vec![]);

        // "a" appears once (first-seen wins)
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id.as_str(), "a");
        assert_eq!(merged[0].name.as_deref(), Some("Alpha First"));
    }

    #[test]
    fn merge_live_server_model_without_curated_preserves_all_server_data() {
        let live = vec![Model::new("gpt-4o")
            .name("GPT-4o".to_string())
            .description("A fast model".to_string())
            .context_length(128000)
            .tools_supported(true)
            .supports_reasoning(false)
            .input_modalities(vec![InputModality::Text, InputModality::Image])];

        let merged = Model::merge_live(live, vec![]);

        assert_eq!(merged.len(), 1);
        let m = &merged[0];
        assert_eq!(m.id.as_str(), "gpt-4o");
        assert_eq!(m.name.as_deref(), Some("GPT-4o"));
        assert_eq!(m.description.as_deref(), Some("A fast model"));
        assert_eq!(m.context_length, Some(128000));
        assert_eq!(m.tools_supported, Some(true));
        assert_eq!(m.supports_reasoning, Some(false));
        assert_eq!(
            m.input_modalities,
            vec![InputModality::Text, InputModality::Image]
        );
    }
}
