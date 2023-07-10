use std::path::Path;

pub(crate) fn path_to_root(path: &Path) -> String {
	let mut parts = Vec::new();
	let mut current = path.parent().unwrap();

	while let Some(parent) = current.parent() {
		if current == parent {
			break;
		}

		parts.push("..");
		current = parent;
	}

	if parts.is_empty() {
		".".into()
	} else {
		parts.join("/")
	}
}
