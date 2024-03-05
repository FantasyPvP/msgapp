use rocket::fs::NamedFile;
use std::path::{Path, PathBuf};

#[get("/assets/style.css")]
pub async fn serve_css() -> Option<NamedFile> {
    NamedFile::open(Path::new("css/output.css")).await.ok()
}

#[get("/public/<file..>", rank = 2)]
pub async fn public_file(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("public").join(file)).await.ok()
}

#[get("/userdata/<file..>", rank = 2)]
// TODO: add a request guard to authenticate client
/*
    - implement a request guard that checks that the client has access to a user's profile
    - the request guard will validate the incoming path, ensuring that the user has access
*/
pub async fn user_data(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("userassets").join(file))
        .await
        .ok()
}
