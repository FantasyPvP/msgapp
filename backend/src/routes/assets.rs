use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};

#[get("/assets/style.css")]
pub async fn serve_css() -> Option<NamedFile> {
    NamedFile::open(Path::new("css/output.css")).await.ok()
}
