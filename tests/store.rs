use larry::Store;

#[test]
fn blobs_append_view_truncate_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let blob = store.blob("data").unwrap();
    assert!(blob.is_empty());
    assert_eq!(blob.view().unwrap().len(), 0);
    assert_eq!(blob.append(b"hello ").unwrap(), 0);
    let before = blob.view().unwrap();
    assert_eq!(blob.append(b"world").unwrap(), 6);
    assert_eq!(&*before, b"hello "); // a view keeps what it saw
    assert_eq!(&*blob.view().unwrap(), b"hello world");
    let mut word = [0u8; 5];
    blob.read_at(6, &mut word).unwrap();
    assert_eq!(&word, b"world");
    blob.sync().unwrap();
    blob.truncate(5).unwrap();
    assert!(blob.truncate(6).is_err()); // never grows by truncation
    drop(blob);
    let blob = store.blob("data").unwrap();
    assert_eq!(blob.read_all().unwrap(), b"hello");
}

#[test]
fn documents_are_replaced_whole() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.read("options").unwrap(), None);
    store.write("options", b"one").unwrap();
    store.write("options", b"two, longer").unwrap();
    assert_eq!(store.read("options").unwrap().unwrap(), b"two, longer");
    // no temporary file left behind, and temporary names aren't valid names
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    assert!(store.read(".options.new").is_err());
    store.remove("options").unwrap();
    store.remove("options").unwrap();
    assert_eq!(store.read("options").unwrap(), None);
}

#[test]
fn nested_stores() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.store("b").unwrap().blob("x").unwrap().append(b"1").unwrap();
    store.store("a").unwrap();
    assert_eq!(store.stores().unwrap(), vec!["a".to_string(), "b".to_string()]);
    assert!(store.has_store("b") && !store.has_store("c"));
    assert!(store.store("../escape").is_err());
    assert!(store.size() >= 1);
    store.store("b").unwrap().destroy().unwrap();
    assert_eq!(store.stores().unwrap(), vec!["a".to_string()]);
}
