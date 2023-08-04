mod fixture;

use std::collections::HashMap;

use fixture::Fixture;

fn options() -> Option<HashMap<String, String>> {
	let mut map = HashMap::new();
	map.insert(
		"MDBOOK_OUTPUT__ANGULAR__PLAYGROUNDS".to_owned(),
		"false".to_owned(),
	);
	Some(map)
}

#[test]
fn test_without_flags() {
	let fixture = Fixture::run_without_build(options());
	let chapter = fixture.chapter1();

	chapter.assert_collapsed(false);
	chapter.assert_is_default_insertion(true);
	chapter.assert_code_block_count(2);
	chapter.assert_has_playground(false);
}

#[test]
fn test_flag_no_insert() {
	let fixture = Fixture::run_without_build(options());
	let chapter = fixture.chapter2();

	chapter.assert_collapsed(false);
	chapter.assert_is_default_insertion(false);
	chapter.assert_code_block_count(2);
	chapter.assert_has_playground(false);
}

#[test]
fn test_flag_uncollapsed_no_playground() {
	let fixture = Fixture::run_without_build(options());
	let chapter = fixture.chapter3();

	chapter.assert_collapsed(false);
	chapter.assert_is_default_insertion(true);
	chapter.assert_code_block_count(2);
	chapter.assert_has_playground(false);
}

#[test]
fn test_flag_collapsed_playground() {
	let fixture = Fixture::run_without_build(options());
	let chapter = fixture.chapter4();

	chapter.assert_collapsed(true);
	chapter.assert_is_default_insertion(true);
	chapter.assert_code_block_count(2);
	chapter.assert_has_playground(true);
}

#[test]
fn test_hide() {
	let fixture = Fixture::run_without_build(options());
	let chapter = fixture.chapter5();

	chapter.assert_collapsed(false);
	chapter.assert_is_default_insertion(true);
	chapter.assert_code_block_count(0);
	chapter.assert_has_playground(false);
}
