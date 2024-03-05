use rocket::{
    futures::{SinkExt, StreamExt},
    http::Status,
    serde::{Deserialize, Serialize},
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

#[get("/home")]
pub fn home(g: AuthTokenGuard) -> Template {
    let messages = vec![
        Message {
            profile_picture: String::from("ayyo"),
            username: String::from("FantasyPvP"),
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
pub fn chat<'r>(g: AuthTokenGuard, ws: WebSocket) -> Channel<'r> {
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
