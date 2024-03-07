use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use chrono::{DateTime, Utc};

use rocket::{
    futures::{channel::mpsc, SinkExt, StreamExt},
    http::Status,
    serde::{Deserialize, Serialize},
    Rocket, State,
    tokio::sync::Mutex,
};

use rocket_db_pools::{
    sqlx::{self, Row},
    Connection,
};

use rocket_dyn_templates::{context, Template};
use rocket_ws::{Channel, Stream, WebSocket};
use sha2::digest::const_oid::Arcs;

use crate::auth::AuthTokenGuard;
use crate::DbInterface;

#[derive(Serialize, Deserialize, Clone)]
pub struct UserMessage {
    pub user_name: String,
    pub content: String,
    pub datetime: String,
}

#[get("/home")]
pub async fn home(_g: AuthTokenGuard, mut db: Connection<DbInterface>) -> Template {
    let messages = sqlx::query!(
        "SELECT u.user_name, m.content, m.datetime FROM User AS u JOIN Message AS m ON m.user_id = u.user_id ORDER BY m.datetime DESC LIMIT 250;"
    ).fetch_all(&mut **db).await.expect("unable to fetch messages from database: TODO: remove this unwrap");

    let messages = messages.into_iter().map(|m| UserMessage {
        user_name: m.user_name.unwrap(),
        content: m.content,
        datetime: {
            let duration = std::time::Duration::from_millis(m.datetime as u64);
            let system_time = std::time::UNIX_EPOCH + duration;
            let datetime: DateTime<Utc> = DateTime::from(system_time);
            datetime.format("%d-%m-%y %H-%M-%S").to_string()
        },
    }).rev().collect::<Vec<_>>();

    // let messages = (0..50).map(|_| UserMessage {
    //     user_name: String::from("zxq5"),
    //     date_time: String::from("05/03/24"),
    //     content: String::from("Panic_Attack444 is a simp. this has been factually confirmed on many occasions and is objectively true"),
    // }).collect::<Vec<_>>();
    Template::render("home", context! { messages })
}

#[get("/chat")]
pub async fn chat<'r>(
    g: AuthTokenGuard,
    ws: WebSocket,
    mut db: Connection<DbInterface>,
    conns: &'r State<WebSocketConnections>,
) -> Channel<'r> {
    let (sender, mut receiver) = mpsc::channel::<UserMessage>(100);
    let AuthTokenGuard(user_id) = g;
    println!("USER_ID: {} CONNECTED", user_id);
    
    conns.connections.lock().await.push((user_id, sender));

    ws.channel(move |stream| {
        let (mut ws_sender, mut ws_receiver) = stream.split();

        let ws_task = async move {
            while let Some(packet) = ws_receiver.next().await {
                let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
                let message = packet.expect("error recieving packet").into_text().unwrap();
                println!("RECEIVED MESSAGE FROM FRONTEND: {}", &message);
                println!("userid {}", user_id);

                if message.len() == 0 {
                    continue;
                }

                sqlx::query!("INSERT INTO Message (user_id, content, datetime)
                    VALUES (?, ?, ?)",
                    user_id, message, current_time
                ).execute(&mut **db).await.expect("failed to insert message into database");
            }

            conns.connections.lock().await.retain(|(id, _)| *id != user_id);
            println!("USER {} DISCONNECTED", user_id);
        };

    
        let channel_task = async move {
            while let Some(msg) = receiver.next().await {
                println!("FOUND NEW MESSAGE IN DATABASE: {}", msg.content);
                match ws_sender.send(serde_json::to_string(&msg).unwrap().into()).await {
                    Ok(_) => {},
                    Err(_) => println!( "failed to send message" ),
                }
            }
        };
        Box::pin(async move {
            let _ = tokio::select! {
                _ = ws_task => {},
                _ = channel_task => {},
            };
            Ok(())
        })
    })
}

pub struct WebSocketConnections {
    pub connections: Arc<Mutex<Vec<(i64, mpsc::Sender<UserMessage>)>>>, // the i64 is a user id for the websocket
                                                                   // TODO: decouple from user ID
}
