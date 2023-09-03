use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PlaygroundInputType {
	#[default]
	String,
	Boolean,
	Number,
	Enum(Vec<String>),
}

impl PlaygroundInputType {
	pub(crate) fn is_string(&self) -> bool {
		*self == PlaygroundInputType::String
	}

	// pub(crate) fn is_boolean(&self) -> bool {
	// 	*self == PlaygroundInputType::Boolean
	// }

	// pub(crate) fn is_number(&self) -> bool {
	// 	*self == PlaygroundInputType::Number
	// }

	pub(crate) fn is_enum(&self) -> bool {
		matches!(self, PlaygroundInputType::Enum(_))
	}
}

#[derive(Debug, PartialEq, Default, Serialize, Deserialize)]
pub(crate) struct PlaygroundInputConfig {
	#[serde(rename = "type", default)]
	type_: PlaygroundInputType,
	#[serde(rename = "default")]
	default_: Option<Value>,
}

pub(super) trait PlaygroundInputConfigExt {
	fn extend(self, config: PlaygroundInputConfig) -> PlaygroundInputConfig;

	fn as_boolean(&self) -> Option<bool>;
	fn into_number(self) -> Option<serde_json::Number>;
	fn into_string(self) -> Option<String>;

	fn get_default(&self) -> Option<&Value>;
}

impl PlaygroundInputConfigExt for PlaygroundInputConfig {
	#[inline]
	fn extend(self, config: PlaygroundInputConfig) -> PlaygroundInputConfig {
		PlaygroundInputConfig {
			default_: self.default_.or(config.default_),
			type_: if self.type_.is_string() && config.type_.is_enum() {
				config.type_
			} else {
				self.type_
			},
		}
	}

	fn as_boolean(&self) -> Option<bool> {
		if let Some(default_) = &self.default_ {
			match default_ {
				Value::Bool(value) => Some(*value),
				Value::String(value) => Some(!value.is_empty()),
				Value::Number(value) => Some(value.as_f64().unwrap_or(0.0) != 0.0),
				Value::Null => Some(false),
				Value::Array(_) | Value::Object(_) => Some(true),
			}
		} else {
			None
		}
	}

	fn into_number(self) -> Option<serde_json::Number> {
		if let Some(Value::Number(value)) = self.default_ {
			Some(value)
		} else {
			None
		}
	}

	fn into_string(self) -> Option<String> {
		if let Some(Value::String(value)) = self.default_ {
			Some(value)
		} else {
			None
		}
	}

	#[inline]
	fn get_default(&self) -> Option<&Value> {
		self.default_.as_ref()
	}
}

impl PlaygroundInputConfigExt for Option<PlaygroundInputConfig> {
	#[inline]
	fn extend(self, config: PlaygroundInputConfig) -> PlaygroundInputConfig {
		if let Some(self_) = self {
			self_.extend(config)
		} else {
			config
		}
	}

	#[inline]
	fn as_boolean(&self) -> Option<bool> {
		self.as_ref().and_then(PlaygroundInputConfigExt::as_boolean)
	}

	#[inline]
	fn into_number(self) -> Option<serde_json::Number> {
		self.and_then(PlaygroundInputConfigExt::into_number)
	}

	#[inline]
	fn into_string(self) -> Option<String> {
		self.and_then(PlaygroundInputConfigExt::into_string)
	}

	#[inline]
	fn get_default(&self) -> Option<&Value> {
		self.as_ref().and_then(PlaygroundInputConfig::get_default)
	}
}

impl PlaygroundInputConfig {
	#[inline]
	pub(super) fn new(
		default_: Option<Value>,
		type_: PlaygroundInputType,
	) -> PlaygroundInputConfig {
		PlaygroundInputConfig { type_, default_ }
	}

	#[inline]
	pub(super) fn boolean() -> PlaygroundInputConfig {
		PlaygroundInputConfig {
			type_: PlaygroundInputType::Boolean,
			default_: None,
		}
	}

	#[inline]
	pub(super) fn number() -> PlaygroundInputConfig {
		PlaygroundInputConfig {
			type_: PlaygroundInputType::Number,
			default_: None,
		}
	}

	#[inline]
	pub(super) fn string() -> PlaygroundInputConfig {
		PlaygroundInputConfig {
			type_: PlaygroundInputType::String,
			default_: None,
		}
	}

	#[inline]
	pub(super) fn from_type(type_: PlaygroundInputType) -> PlaygroundInputConfig {
		PlaygroundInputConfig {
			type_,
			default_: None,
		}
	}

	#[inline]
	pub(super) fn from_default<T: Into<Value>>(default_: T) -> PlaygroundInputConfig {
		let default_ = default_.into();

		PlaygroundInputConfig {
			type_: match &default_ {
				Value::Bool(_) => PlaygroundInputType::Boolean,
				Value::Number(_) => PlaygroundInputType::Number,
				_ => PlaygroundInputType::String,
			},
			default_: Some(default_),
		}
	}

	#[inline]
	pub(super) fn get_type(self) -> PlaygroundInputType {
		self.type_
	}
}

pub(crate) struct PlaygroundInput {
	pub(crate) name: String,
	pub(crate) description: Option<String>,
	pub(crate) config: PlaygroundInputConfig,
}

pub(crate) struct PlaygroundAction {
	pub(crate) name: String,
	pub(crate) description: String,
}

pub(crate) struct Playground {
	pub(crate) inputs: Vec<PlaygroundInput>,
	pub(crate) actions: Vec<PlaygroundAction>,
}
