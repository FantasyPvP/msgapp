use rocket::{
    fairing::{Fairing, Info, Kind},
    futures::{channel::mpsc, SinkExt, StreamExt},
    http::Status,
    serde::{Deserialize, Serialize},
    Rocket,
};
use rocket_dyn_templates::{context, Template};
use rocket_ws::{Channel, Stream, WebSocket};

use crate::auth::AuthTokenGuard;

#[derive(Serialize, Deserialize)]
struct Message {
    profile_picture: String,
    username: String,
    date: String,
    content: String,
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
        Ok(rocket)
    }
}

#[get("/home")]
pub fn home(g: AuthTokenGuard) -> Template {
    let messages = vec![
        Message {
            profile_picture: String::from("ayyo"),
            username: String::from("fantasypvp"),
            date: String::from("05/03/24"),
            content: String::from("Panic_Attack444 is a simp. this has been factually confirmed on many occasions and is objectively true"),
        },
        Message {
            profile_picture: String::from("idk"),
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
        Message {
            profile_picture: String::from("idk"),
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
        Message {
            profile_picture: String::from("idk"),
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
        Message {
            profile_picture: String::from("idk"),
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
    ];
    Template::render("home", context! { messages })
}

#[get("/chat")]
pub async fn chat<'r>(g: AuthTokenGuard, ws: WebSocket) -> Channel<'r> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = stream.next().await {
                println!("recieved: {}", message.as_ref().expect("failed").clone());
                let _ = stream.send("test2".into()).await;
            }
            Ok(())
        })
    })
}
