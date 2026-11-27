use super::export::standalone;

#[test]
fn markdown_reader_html_export_is_offline_and_does_not_write_source() {
    let dir = tempfile::tempdir().expect("create HTML export fixture directory");
    let path = dir.path().join("reference.md");
    let src = "# 中文标题\n\n| A | B |\n| --- | --- |\n| a | b |\n\n![local](local.png)\n\n![remote](https://example.invalid/remote.png)\n\n<script>throw new Error()</script>\n";
    std::fs::write(&path, src).expect("write HTML export source fixture");
    image::RgbImage::new(1, 1)
        .save(dir.path().join("local.png"))
        .expect("save local PNG export fixture");
    let html = standalone(&path, src).expect("export standalone offline HTML");
    assert!(html.contains("<h1 id=\"中文标题\">中文标题</h1>"));
    assert!(html.contains("href=\"#中文标题\""));
    assert!(html.contains("<table>"));
    assert!(html.contains("<img src=\"data:image/png;base64,"));
    assert!(!html.contains("<img src=\"https://"));
    assert!(!html.contains("<script>"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read unchanged export source"),
        src
    );
}
