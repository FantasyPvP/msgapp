use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use rocket::{
    futures::{channel::mpsc, SinkExt, StreamExt},
    http::Status,
    serde::{Deserialize, Serialize},
    Rocket, State,
};
use rocket_dyn_templates::{context, Template};
use rocket_ws::{Channel, Stream, WebSocket};
use sha2::digest::const_oid::Arcs;

use crate::auth::AuthTokenGuard;

#[derive(Serialize, Deserialize)]
struct Message {
    username: String,
    date: String,
    content: String,
}

#[get("/home")]
pub fn home(g: AuthTokenGuard) -> Template {
    let messages = vec![
        Message {
            username: String::from("fantasypvp"),
            date: String::from("05/03/24"),
            content: String::from("Panic_Attack444 is a simp. this has been factually confirmed on many occasions and is objectively true"),
        },
        Message {
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
        Message {
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
        Message {
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
        Message {
            username: String::from("idk"),
            date: String::from("idk"),
            content: String::from("idk"),
        },
    ];
    Template::render("home", context! { messages })
}

#[get("/chat")]
pub async fn chat<'r>(
    g: AuthTokenGuard,
    ws: WebSocket,
    conns: &State<WebSocketConnections>,
) -> Channel<'r> {
    let (sender, mut receiver) = mpsc::channel::<Message>(100);

    ws.channel(move |mut stream| {
        let ws_sender = sender.clone();

        let ws_task = async move {
            while let Some(message) = stream.next().await {
                println!("recieved: {}", message.as_ref().expect("failed").clone());
            }
            Ok(())
        };

        let channel_task = async move {
            while let Some(msg) = receiver.next().await {
                println!("receiver from channel: {}", msg.content);
                let _ = stream.send(msg.content.into()).await;
            }
        };

        Box::pin(async move {
            loop {
                tokio::select! {
                    _ = ws_task => {},
                    _ = channel_task => {},
                }
            }
        })
    })
}

pub struct WebSocketConnections {
    pub connections: Arc<Mutex<Vec<(i64, mpsc::Sender<String>)>>>, // the i64 is a user id for the websocket
                                                                   // TODO: decouple from user ID
}
