use rand::{thread_rng, Rng};
use reqwest::header::ACCESS_CONTROL_REQUEST_HEADERS;
use rocket::http::CookieJar;
use rsa::rand_core::OsRng;
use sha2::digest::typenum::False;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::read_to_string;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{event, Level};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use rocket::{
    http::{Cookie, SameSite, Status},
    request::{self, FromRequest, Outcome, Request},
    response::{Redirect, Responder},
    serde::{json::Json, Deserialize, Serialize},
    State,
};
use rocket_db_pools::{
    sqlx::{self, FromRow, Row},
    Connection,
};

use rocket_dyn_templates::{context, Template};

use jsonwebtoken::{
    decode, encode, errors::Error, Algorithm, DecodingKey, EncodingKey, Header, TokenData,
    Validation,
};

use crate::DbInterface;

/*
AUTH TOKEN GUARD - uses a cookie to check users are authenticated before proceeding to a given endpoint
*/

// this request guard is used to ensure that a user is authenticated when making any requests

/*
LOGIN LOGIC
*/

#[get("/login")]
pub fn user_login_page() -> Template {
    Template::render("login", context! { title: "test"})
}

#[derive(Serialize)]
struct LoginResponse {
    redirect: Option<String>,
}

#[post("/login", data = "<form>")]
pub async fn api_login<'a>(
    mut db: Connection<DbInterface>,
    form: Json<LoginForm>,
    _c: BrowserClient,
    jar: &CookieJar<'_>,
) -> Option<Redirect> {
    if login_logic(jar, form, db).await {
        Some(Redirect::to("/"))
    } else {
        None
    }
}

#[post("/login", data = "<form>", rank = 2)]
pub async fn api_login_nonbrowser(
    mut db: Connection<DbInterface>,
    form: Json<LoginForm>,
    jar: &CookieJar<'_>,
) -> String {
    if login_logic(jar, form, db).await {
        String::from("ok")
    } else {
        String::from("rejected")
    }
}

async fn login_logic(
    jar: &CookieJar<'_>,
    form: Json<LoginForm>,
    mut db: Connection<DbInterface>,
) -> bool {
    println!("logging in...");

    let (pass_hash, user_id): (String, i64) = match sqlx::query!(
        "SELECT user_id, pass_hash FROM User WHERE user_name = ?",
        form.username
    )
    .fetch_optional(&mut **db)
    .await
    .unwrap()
    {
        Some(record) => (
            record.pass_hash.expect("all users should have a pass hash"),
            record.user_id,
        ),
        _ => return false,
    };

    let hash = PasswordHash::new(&pass_hash).expect("unable to generate hash");

    if let Ok(_) = Argon2::default().verify_password(form.password.as_bytes(), &hash) {
        let token = SessionToken::new(user_id);

        sqlx::query!(
            "INSERT INTO SessionToken (token, created_at, expires_at, user_id) VALUES (?, ?, ?, ?)",
            token.token,
            token.created_at,
            token.expires_at,
            token.user_id
        )
        .execute(&mut **db)
        .await
        .unwrap();

        jar.add_private(("session_token", token.token));

        true
    } else {
        false
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct LoginForm {
    username: String,
    password: String,
}

struct BrowserClient;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BrowserClient {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if request.headers().get_one("Referer").is_some() {
            Outcome::Success(BrowserClient)
        } else {
            Outcome::Forward(Status::Forbidden)
        }
    }
}

#[post("/signup", data = "<req>")]
pub async fn signup(
    mut db: Connection<DbInterface>,
    req: Json<SignupData>,
) -> Json<SignupResponse> {
    match validate_access_token(&req.token) {
        Ok(false) => {
            return Json(SignupResponse {
                token: None,
                error: Some(String::from("Access token invalid or expired")),
            })
        }
        Err(e) => {
            event!(Level::WARN, "{}", e);
            return Json(SignupResponse {
                token: None,
                error: Some(String::from("Access token invalid or expired")),
            });
        }
        _ => (),
    };

    if let Err(e) = validate_username(&req.username) {
        return Json(SignupResponse {
            token: None,
            error: Some(e),
        });
    } else if let Err(e) = validate_password(&req.password) {
        return Json(SignupResponse {
            token: None,
            error: Some(e),
        });
    }

    let now = SystemTime::now();
    let join_date = now
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs() as i64;

    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let pass_hash = argon
        .hash_password(req.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query!(
        "
            INSERT INTO User (user_name, pass_hash, join_date, display_name) VALUES
            (?, ?, ?, ?)
        ",
        req.username,
        pass_hash,
        join_date,
        req.displayname
    )
    .execute(&mut **db)
    .await;

    Json(SignupResponse {
        token: Some(generate_session_token().to_string()),
        error: None,
    })
}

#[get("/signup")]
pub fn user_signup_page() -> Template {
    Template::render("signup", context! {})
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct SignupData {
    token: String,
    username: String,
    displayname: Option<String>,
    password: String,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct SignupResponse {
    token: Option<String>,
    error: Option<String>,
}

fn validate_access_token(tk: &str) -> Result<bool, std::io::Error> {
    Ok(read_to_string("./access_tokens.txt")?
        .lines()
        .collect::<Vec<&str>>()
        .contains(&tk))
}

fn generate_session_token() -> usize {
    20 // TODO: add an actual algorithm for this
}

fn validate_username(username: &str) -> Result<(), String> {
    if !username.is_ascii() {
        return Err(String::from("Username must contain only ASCII characters"));
    }
    if username.len() > 30 || username.len() < 3 {
        return Err(String::from("Username must be between 3 and 30 characters"));
    }
    // TODO: add additional validation checks

    Ok(())
}

fn validate_password(password: &str) -> Result<(), String> {
    if !password.is_ascii() {
        return Err(String::from("Password must contain only ASCII characters"));
    }
    if password.len() > 30 || password.len() < 8 {
        return Err(String::from("Password must be between 8 and 30 characters"));
    }
    // TODO: add additional validation checks

    Ok(())
}

#[derive(Debug)]
pub struct AuthTokenGuard(i64); // the contained data is the user_id

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    user_id: i64,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthTokenGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(cookie) = request.cookies().get_private("session_token") {
            let pool = match request.guard::<&State<DbInterface>>().await {
                Outcome::Success(pool) => pool,
                _ => return Outcome::Error((Status::InternalServerError, ())),
            };
            let value = cookie.value();

            if let Some(token) = sqlx::query_as!(SessionToken,
                "SELECT token_id, token, created_at, expires_at, user_id FROM SessionToken WHERE token = ?",
                value
            ).fetch_optional(&***pool).await.expect("SQL QUERY FAILED") {
                return Outcome::Success(AuthTokenGuard(token.user_id))
            }
        }
        Outcome::Error((rocket::http::Status::Unauthorized, ()))
    }
}

#[derive(FromRow)]
pub struct SessionToken {
    token_id: Option<i64>,
    token: String,
    created_at: i64,
    expires_at: Option<i64>, // time stored as a unix timestamp
    user_id: i64,
}

impl SessionToken {
    pub fn new(user_id: i64) -> SessionToken {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let expiry = Duration::from_secs(7 * 24 * 60 * 60);

        let random_value: u32 = thread_rng().gen();
        let token = format!("{}-{}", current_time.as_secs(), random_value);
        let hashed = format!("{:x}", Sha256::digest(token.as_bytes()));

        println!("{}", hashed);

        SessionToken {
            token_id: None,
            token: String::new(),
            created_at: current_time.as_secs() as i64,
            expires_at: Some((current_time + expiry).as_secs() as i64),
            user_id,
        }
    }
}
