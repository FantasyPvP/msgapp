use rand::{thread_rng, Rng};
use reqwest::header::ACCESS_CONTROL_REQUEST_HEADERS;
use rocket::http::CookieJar;
use sha2::digest::typenum::False;
use sha2::{Digest, Sha256};
use std::arch::x86_64::_CMP_TRUE_UQ;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
