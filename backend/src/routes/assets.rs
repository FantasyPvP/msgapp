use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};

#[get("/assets/style.css")]
pub async fn serve_css() -> Option<NamedFile> {
    NamedFile::open(Path::new("css/output.css")).await.ok()
}

#[get("/public/<file..>", rank = 2)]
pub async fn file(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("public").join(file)).await.ok()
}
