use std::path::{Path, PathBuf};
use rocket::fs::NamedFile;
use rocket_dyn_templates::{context, Template};
use std::fs;
use rocket::http::Status;
use rocket::serde::{Deserialize, Serialize};
use tracing::{event, Level};
use toml;

#[derive(Serialize, Debug)]
struct Pack {
    id: String,
    name: String,
    description: String,
}

#[derive(Serialize, Debug)]
struct Section {
    title: String,
    description: String,
    options: Vec<Pack>,
    allow_none: bool,
}

#[derive(Deserialize)]
struct SectionConfig {
    title: String,
    description: String,
    allow_none: bool,
}

#[get("/")]
pub fn endpoint_packs() -> Template {

    // iterates over packs directory and finds all the folders, getting the folder name, pack name, and the description of the pack
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

                        println!("{}", dir_name);

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

#[get("/builder")]
pub fn endpoint_pack_builder() -> Result<Template, Status> {

    let mut sections: Vec<Section> = Vec::new();

    let paths = match fs::read_dir("packbuilder") {
        Ok(dir) => dir,
        Err(_) => {
            event!(Level::ERROR, "Failed to read packs directory");
            return Err(Status::InternalServerError); },
    };

    let sections: Vec<Section> = paths
        .filter_map(|p| p.ok())
        .filter(|dir| dir.path().is_dir())
        .filter_map(|dir| {

            println!("Found Section: {}", dir.file_name().to_str().unwrap());

            // open config file for section
            match toml::from_str::<SectionConfig>(match &fs::read_to_string(dir.path().join("section.toml")) {
                Ok(s) => s,
                Err(_) => {
                    event!(Level::ERROR, "Failed to read section config file for {}", dir.file_name().to_str().unwrap());
                    return None
                }
            }) {
                Ok(config) => {
                    let mut sect = Section {
                        title: config.title,
                        description: config.description,
                        options: Vec::new(),
                        allow_none: config.allow_none
                    };

                    for pack in fs::read_dir(dir.path()).unwrap().filter_map(Result::ok) {
                        if pack.path().is_dir() {
                            let pack_path = pack.path();
                            let pack_name = pack.file_name().into_string().unwrap();

                            println!("Found Pack: {}", pack.file_name().to_str().unwrap());

                            if let Some(txt_file) = fs::read_dir(pack_path).unwrap()
                                .filter_map(Result::ok)
                                .find(|entry| {
                                    entry.file_name()
                                        .to_str()
                                        .map(|s| s.ends_with(".txt"))
                                        .unwrap_or(false)
                                }) {

                                println!("Found txt file: {}", txt_file.path().display());

                                println!("{}", pack_name);

                                sect.options.push(Pack {
                                    id: pack_name,
                                    name: txt_file.path().file_stem().unwrap().to_str().unwrap().to_string(),
                                    description: fs::read_to_string(txt_file.path()).unwrap()
                                })
                            }
                        }
                    }
                    println!("{:#?}", sect);
                    Some(sect)
                }
                Err(e) => {
                    event!(Level::ERROR, "Failed to parse config file: {}", e);
                    None
                }
            }
        })
        .collect();


    // for p in paths {
    //     if let Ok(dir) = p {
    //         if let Ok(dir_name) = dir.file_name().into_string() {
    //             let dir_path = dir.path();
    //
    //             if dir_path.is_dir() && !dir_name.starts_with('.') {
    //                 println!("Found Section: {}", dir_name);
    //
    //                 let mut section = Section {
    //                     title: dir_name,
    //                     description: "Packs to build".to_string(),
    //                     options: Vec::new()
    //                 };
    //
    //                 if let Some(txt_file) = fs::read_dir(dir_path).unwrap()
    //                     .filter_map(Result::ok)
    //                     .find(|entry| {
    //                         entry.file_name()
    //                             .to_str()
    //                             .map(|s| s.ends_with(".txt"))
    //                             .unwrap_or(false)
    //                     }) {
    //
    //                     println!("Found txt file: {}", txt_file.path().display());
    //
    //                     println!("{}", dir_name);
    //
    //                     packs.push(Pack {
    //                         id: dir_name,
    //                         name: txt_file.path().file_stem().unwrap().to_str().unwrap().to_string(),
    //                         description: fs::read_to_string(txt_file.path()).unwrap()
    //                     })
    //                 }
    //             }
    //         }
    //     }
    // }
    //
    // sections.push(Section {
    //     title: "Packs".to_string(),
    //     description: "Packs to build".to_string(),
    //     options: packs
    // });


    Ok(Template::render("packbuilder/main", context! { sections: sections }))
}

















































