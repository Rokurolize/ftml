use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn wikidot_fixture_dirs(path: &Path, fixtures: &mut BTreeSet<String>) {
    for entry in fs::read_dir(path).expect("read tree-test directory") {
        let path = entry.expect("read tree-test entry").path();
        if path.is_dir() {
            wikidot_fixture_dirs(&path, fixtures);
        } else if path.file_name().is_some_and(|name| name == "wikidot.html") {
            fixtures.insert(
                path.parent()
                    .expect("fixture has a parent")
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .expect("fixture is inside the repository")
                    .to_string_lossy()
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
}

#[test]
fn parity_index_lists_each_wikidot_fixture_once() {
    let documented: Vec<_> = include_str!("../docs/ParityTests.md")
        .lines()
        .filter_map(|line| line.split('|').nth(2))
        .map(|cell| cell.trim().trim_matches('`'))
        .filter(|path| path.starts_with("test/"))
        .collect();
    let documented_set: BTreeSet<_> = documented.iter().copied().collect();
    let mut fixtures = BTreeSet::new();
    wikidot_fixture_dirs(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test").as_path(),
        &mut fixtures,
    );

    assert_eq!(
        documented.len(),
        documented_set.len(),
        "duplicate parity index rows"
    );
    assert_eq!(
        documented_set,
        fixtures.iter().map(String::as_str).collect::<BTreeSet<_>>()
    );
}
