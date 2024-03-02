use crate::DbInterface;
use rocket::{
    http::Status,
    request::{self, FromRequest, Outcome, Request},
    serde::{json::Json, Deserialize, Serialize},
};
use rsa::rand_core::OsRng;
use std::fs::read_to_string;
use tracing::{event, Level};

use rocket_db_pools::{
    sqlx::{self, Row},
    Connection,
};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use std::time::{SystemTime, UNIX_EPOCH};

pub struct AuthTokenGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthTokenGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(cookie) = request.cookies().get("auth_token_cookie_name") {
            if cookie.value() == "expected_auth_token_value" {
                return Outcome::Success(AuthTokenGuard);
            }
        }
        Outcome::Error((rocket::http::Status::Unauthorized, ()))
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
