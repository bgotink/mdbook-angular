use std::path::Path;

pub(crate) fn path_to_root<P: AsRef<Path>>(path: P) -> String {
	let mut parts = Vec::new();
	let mut current = path.as_ref().parent().unwrap();

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

#[cfg(test)]
mod test {
	use super::path_to_root;

	#[test]
	fn test_path_to_root() {
		assert_eq!("..", path_to_root("lorem/ipsum.html"));
		assert_eq!("../..", path_to_root("lorem/ipsum/dolor.html"));
		assert_eq!(".", path_to_root("lorem.html"));
	}
}
