use std::path::Path;

use judou_lib::ingest::epub::{locate_opf_path, read_package_document};

#[test]
fn reference_epub_locates_opf_from_container() {
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    assert_eq!(locate_opf_path(container).unwrap(), "content.opf");
}

#[test]
fn reference_epub_reads_metadata_manifest_and_spine_without_chapters() {
    let package = read_package_document(Path::new(
        "../fixtures/epub/Inside the Box - David Epstein.epub",
    ))
    .unwrap();

    assert_eq!(package.opf_path, "content.opf");
    assert_eq!(package.metadata.title.as_deref(), Some("Inside the Box"));
    assert_eq!(package.metadata.author.as_deref(), Some("David Epstein"));
    assert_eq!(package.metadata.language.as_deref(), Some("en"));
    assert_eq!(package.ncx_href.as_deref(), Some("toc.ncx"));

    let first_five_hrefs: Vec<&str> = package
        .spine
        .iter()
        .take(5)
        .map(|item| item.href.as_str())
        .collect();
    assert_eq!(
        first_five_hrefs,
        vec![
            "titlepage.xhtml",
            "OEBPS/c9.xhtml",
            "OEBPS/cP.xhtml",
            "OEBPS/cZ.xhtml",
            "OEBPS/c2A.xhtml"
        ]
    );
    assert_eq!(package.spine.len(), 45);
}
