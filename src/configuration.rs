use std::error::Error;
use std::fmt::{self, Display};
use std::path::Path;
use std::result::Result;
use std::str::FromStr;

use inquire::{Autocomplete, CustomUserError, Confirm, Select, Text, autocompletion::Replacement};

use crate::registry::{DevOption, StringDevOption};

#[derive(Clone, Debug, PartialEq, Default)]
struct ProposalsAutocomplete(Vec<String>);

impl ProposalsAutocomplete {
    fn new(default_value: &String, values: &[String]) -> ProposalsAutocomplete {
        if default_value.is_empty() || values.contains(default_value) {
            ProposalsAutocomplete(values.into())
        } else {
            let mut all_values: Vec<String> = values.into();
            all_values.insert(0, default_value.clone());
            ProposalsAutocomplete(all_values)
        }
    }
}

impl Autocomplete for ProposalsAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        let ProposalsAutocomplete(proposals) = self;
        let input_lower = input.to_lowercase();
        let suggestions =
            proposals.iter()
            .filter_map(|s| if s.to_lowercase().starts_with(&input_lower) { Some(s.clone()) } else { None })
            .collect();
        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        Ok(highlighted_suggestion.or_else(|| {
            let suggestions = self.get_suggestions(input).ok()?;
            if let [suggestion] = suggestions.as_slice() {
                Some(suggestion.clone())
            } else {
                None
            }
        }))
    }
}

pub trait TemplateContext {
    fn render_file<P: AsRef<Path>>(&self, filename: P, buffer: &str) -> Result<usize, Box<dyn Error>>;
}

pub trait DisplayPrompt {
    type Item;

    fn display_prompt(&self) -> Result<Self::Item, Box<dyn Error>>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum DevOptionPromptValue {
    String(String),
    Boolean(bool),
}

impl Display for DevOptionPromptValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DevOptionPromptValue::Boolean(value) => write!(f, "{}", value),
            DevOptionPromptValue::String(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevOptionPrompt<'t> {
    inner: &'t DevOption,
    name: &'t str,
}

impl<'t> DevOptionPrompt<'t> {
    pub fn new(name: &'t str, dev_option: &'t DevOption) -> DevOptionPrompt<'t> {
        DevOptionPrompt {
            inner: dev_option,
            name,
        }
    }
}

impl<'t> DisplayPrompt for DevOptionPrompt<'t> {
    type Item = DevOptionPromptValue;

    fn display_prompt(&self) -> Result<Self::Item, Box<dyn Error>> {
        let dev_option = self.inner;
        let default = dev_option.configured_default();

        match dev_option {
            DevOption::Boolean { description, .. } => {
                let message = description.as_ref().map_or_else(|| format!("Include {}?", self.name), |s| s.clone());
                let default_value = bool::from_str(&default)?;
                let result =
                    Confirm::new(&message)
                    .with_default(default_value)
                    .prompt()?;
                let value = DevOptionPromptValue::Boolean(result);

                Ok(value)
            },
            DevOption::String(StringDevOption::EnumValues { description, r#enum, .. }) => {
                let message = description.as_ref().map_or_else(|| format!("Choose value for {}:", self.name), |s| s.clone());
                let options = r#enum.iter().collect();
                let start = r#enum.iter().position(|s| *s == default).unwrap_or_default();
                let result = Select::new(&message, options).with_starting_cursor(start).prompt()?;
                let value = DevOptionPromptValue::String(result.clone());

                Ok(value)
            },
            DevOption::String(StringDevOption::Proposals { description, proposals, .. }) => {
                let message = description.as_ref().map_or_else(|| format!("What value for {}?", self.name), |s| s.clone());
                let text_prompt = if let Some(values) = proposals.as_ref().filter(|&p| !p.is_empty()) {
                    let autocomplete = ProposalsAutocomplete::new(&default, values);

                    Text::new(&message).with_default(&default).with_autocomplete(autocomplete)
                } else {
                    Text::new(&message).with_default(&default)
                };
                let result = text_prompt.prompt()?;
                let value = DevOptionPromptValue::String(result);

                Ok(value)
            },
        }
    }
}
