use std::path::{Path, PathBuf};
use rocket::fs::NamedFile;
use rocket_dyn_templates::{context, Template};
use std::fs;
use rocket::serde::Serialize;

#[derive(Serialize)]
struct Pack {
    id: String,
    name: String,
    description: String,
}

#[get("/")]
pub fn endpoint_packs() -> Template {

    // scan "packs" folder in root directory and find subdirectories
    // get the name of each subdirectory
    // get the txt file in each subdirectory
    let mut packs: Vec<Pack> = Vec::new();

    let paths = fs::read_dir("packs").unwrap();

    for p in paths {
        if let Ok(dir) = p {
            if let Ok(dir_name) = dir.file_name().into_string() {
                let dir_path = dir.path();

                if dir_path.is_dir() {
                    println!("Found Pack: {}", dir_name);

                    if let Some(txt_file) = fs::read_dir(dir_path).unwrap()
                        .filter_map(Result::ok)
                        .find(|entry| {
                            entry.file_name()
                                .to_str()
                                .map(|s| s.ends_with(".txt"))
                                .unwrap_or(false)
                        }) {

                        println!("Found txt file: {}", txt_file.path().display());

                        packs.push(Pack {
                            id: dir_name,
                            name: txt_file.path().file_stem().unwrap().to_str().unwrap().to_string(),
                            description: fs::read_to_string(txt_file.path()).unwrap()
                        })
                    }
                }
            }
        }
    }

    Template::render("packs/home", context! { packs: packs })
}

#[get("/<file..>")]
pub async fn endpoint_pack_assets(file: PathBuf) -> Option<NamedFile> {
    println!("{}", PathBuf::from("packs").join(&file).display());

    NamedFile::open(PathBuf::from("packs").join(file)).await.ok()
}

