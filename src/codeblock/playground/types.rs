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

pub(crate) trait PlaygroundInputConfigExt {
	fn extend(self, config: PlaygroundInputConfig) -> PlaygroundInputConfig;
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
	pub(super) fn from_type(type_: PlaygroundInputType) -> PlaygroundInputConfig {
		PlaygroundInputConfig {
			type_,
			default_: None,
		}
	}

	#[inline]
	pub(super) fn from_default(default_: Value) -> PlaygroundInputConfig {
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

	#[inline]
	pub(super) fn get_default(self) -> Option<Value> {
		self.default_
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
