use dotenv::dotenv;
use rand;

use rocket::{futures::{
    channel::mpsc::Sender,
    SinkExt,
}, tokio::sync::Mutex, response::{content::RawHtml, Redirect}, serde::{json::Json, Deserialize, Serialize}, fairing::{Fairing, Info, Kind}, Request, Rocket, Orbit};

use rocket_cors::{AllowedOrigins, CorsOptions};
use rocket_db_pools::{
    sqlx::{self, Row},
    Connection, Database,
};
use rocket_dyn_templates::{context, Template};

use std::{
    collections::HashMap,
    env,
    str,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use figment::{providers::Env, Figment};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rocket::fs::TempFile;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};

mod auth;
mod routes;
use auth::AuthTokenGuard;
use crate::routes::messenger::UserMessage;

#[macro_use]
extern crate rocket;

#[derive(Database)]
#[database("CrySrv-DB")]
struct DbInterface(sqlx::SqlitePool);

#[launch]
async fn launch() -> _ {
    dotenv().ok();

    let allowed_origins = AllowedOrigins::some_exact(&[
        "https://localhost:8000",
        "http://localhost:8000",
        "https://fantasypvp.uk",
    ]);

    let options = CorsOptions {
        allowed_origins,
        ..Default::default()
    }
    .to_cors()
    .expect("error while building CORS");

    if env::args()
        .into_iter()
        .collect::<Vec<String>>()
        .contains(&String::from("runtest"))
    {
        symmetric_encryption_test();
        asymmetric_encryption_test();
        password_hashing_test();
    }

    println!("tests passed");

    let secret_key = env::var("JWT_TOKEN").expect("token not found in .env");
    let figment = rocket::Config::figment().merge(("secret_key", secret_key));

    rocket::custom(figment)
        // .attach(options)
        .attach(DbInterface::init())
        .attach(Template::fairing())
        .manage(routes::messenger::WebSocketConnections {
            connections: Arc::new(Mutex::new(Vec::<(i64, Sender<UserMessage>)>::new())),
        })
        .attach(RealTimeMessenger)
        .mount("/api", routes![
            gethtml,
            test,
            auth::api_login,
            auth::api_login_nonbrowser,
            auth::signup,
            routes::messenger::chat,
        ])
        .mount("/", routes![
            auth::user_login_page,
            auth::user_signup_page,
            index,
            routes::messenger::home,
            routes::assets::serve_css,
            routes::assets::public_file,
            routes::assets::user_data,
            routes::assets::favicon,
            routes::revision_presentation::index,
            routes::revision_presentation::check_ans,
        ])
        .mount("/packs", routes![
            routes::packs::endpoint_packs,
            routes::packs::endpoint_pack_assets,
            routes::packs::endpoint_pack_builder,
        ])
        .register("/", catchers![not_found, internal_error, not_authorized])
}

fn symmetric_encryption_test() {
    // testing symmetric encryption.

    let key = Aes256Gcm::generate_key(OsRng);

    let key: &[u8; 32] = &[42; 32];
    let key: &Key<Aes256Gcm> = key.into();

    let key: &[u8] = &[42; 32];
    let key: [u8; 32] = key.try_into().unwrap();

    let key = Key::<Aes256Gcm>::from_slice(&key);

    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher.encrypt(&nonce, b"hello world".as_ref()).unwrap();
    let plaintext = cipher.decrypt(&nonce, ciphertext.as_ref()).unwrap();

    assert_eq!(&b"hello world"[..], &plaintext[..]);
}

fn asymmetric_encryption_test() {
    // testing asymmetric encryption.

    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pub_key = RsaPublicKey::from(&priv_key);

    let data = b"hello world";

    let enc_data = pub_key
        .encrypt(&mut rng, Pkcs1v15Encrypt, &data[..])
        .expect("failed to encrypt");
    let dec_data = priv_key
        .decrypt(Pkcs1v15Encrypt, &enc_data)
        .expect("failed to decrypt");

    assert_eq!(&data[..], &dec_data[..]);
}

fn password_hashing_test() {
    // testing password hashing

    let password = b"hello password!";

    let salt = SaltString::generate(&mut OsRng);

    let argon = Argon2::default();

    let hash = argon.hash_password(password, &salt).unwrap().to_string();
    let parsed_hash = PasswordHash::new(&hash).unwrap();

    assert!(Argon2::default()
        .verify_password(password, &parsed_hash)
        .is_ok());
}

#[get("/test")]
fn test(g: AuthTokenGuard) -> &'static str {
    "test"
}

#[get("/html")]
fn gethtml() -> RawHtml<&'static str> {
    RawHtml("<h1>bruh</h1>")
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml("<h1>fantasypvp.uk</h1>")
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    Template::render("other/404", context! {})
}

#[catch(500)]
fn internal_error() -> Template {
    Template::render("other/500", context! {})
}

#[catch(401)]
fn not_authorized() -> Redirect {
    Redirect::to("/login")
}

pub struct RealTimeMessenger;

#[rocket::async_trait]
impl Fairing for RealTimeMessenger {
    fn info(&self) -> Info {
        Info {
            name: "Database Polling for messenger",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(
        &self,
        rocket: Rocket<rocket::Build>,
    ) -> Result<Rocket<rocket::Build>, Rocket<rocket::Build>> {
        let pool = rocket.state::<DbInterface>().expect("failed");
        let managed_channels = rocket.state::<routes::messenger::WebSocketConnections>().expect("failed");
        let mut connection = pool.acquire().await.unwrap().detach();
        let channels: Arc<Mutex<_>> = Arc::clone(&managed_channels.connections);

        tokio::spawn(async move {
            let mut time: i64;
            
            loop {
                time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                // TODO: add channels
                let messages = sqlx::query!(
                    "
                    SELECT m.content, m.datetime, u.user_name 
                    FROM User AS u 
                    JOIN Message AS m ON m.user_id = u.user_id
                    WHERE m.datetime > ?
                    ORDER BY m.datetime DESC
                    LIMIT 100;", time
                )
                .fetch_all(&mut connection)
                .await
                .unwrap();

                for m in messages {
                    let msg = routes::messenger::UserMessage {
                        user_name: m.user_name.unwrap(),
                        datetime: m.datetime.to_string(),
                        content: m.content,
                    };
                    let mut guard = channels.lock().await;
                    for c in guard.iter_mut() {
                        c.1.send(msg.clone()).await.unwrap();
                    }
                }
            }
        });
        Ok(rocket)
    }
}



















