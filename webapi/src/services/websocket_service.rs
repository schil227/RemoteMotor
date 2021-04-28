extern crate ws;

use std::thread;
use std::sync::RwLock;
use std::sync::Arc;
use std::time::Duration;
use std::cmp::Ordering;

use ws::{ Handler, Sender, Handshake, Result, Message, CloseCode};
use serde_json;
use serde::{Serialize,Deserialize};
use chrono::Utc;
use chrono::DateTime;

use crate::models::command_models::CommandData;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ServerState{
    AcceptingInput,
    Warning,
    Locked
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    state: ServerState,
    command: CommandData
}

pub struct WebSocketHandler {
    _out: Sender,
    last_ping: Arc<RwLock<DateTime<Utc>>>
}

impl Handler for WebSocketHandler {
    fn on_open(&mut self, _: Handshake) -> Result<()>{
        Ok(())
    }

    fn on_message(&mut self, _: Message) -> Result<()>{
        let mut last_ping = self.last_ping.write().unwrap();

        println!("Message received on socket");

        //update connection
        *last_ping = Utc::now();

        Ok(())
    }
}

pub struct WebSocketServer{
    pub server_state: Arc<RwLock<WebSocketMessage>>
}

impl<'a> WebSocketServer{
    pub fn new() -> WebSocketServer{
        WebSocketServer{
            server_state: Arc::new(RwLock::new(WebSocketMessage{
                state: ServerState::AcceptingInput,
                command: CommandData::new()
            }))
        }
    }

    pub fn set_server_state(&mut self, state: ServerState){
        let mut message = self.server_state.write().unwrap();
        message.state = state;
    }

    pub fn set_command_data(&mut self, data: &CommandData){
        let mut message = self.server_state.write().unwrap();
        message.command.copy_from(data);
    }
}

pub fn run(websocket_message : Arc<RwLock<WebSocketMessage>>){
    ws::listen("192.168.1.248:8001", |out| {
        let out_copy = out.clone();
        
        let websocket_message_copy = Arc::clone(&websocket_message);
        let last_ping = Arc::new(RwLock::new(Utc::now()));

        let last_ping_copy = Arc::clone(&last_ping);

        thread::spawn(move || {
            state_change_listener(websocket_message_copy, last_ping_copy, out_copy);
        });

        WebSocketHandler{
            _out: out,
            last_ping: last_ping
        }
    })
    .expect("Failed to create websocket server on ::8001")
}

fn state_change_listener(ws_msg: Arc<RwLock<WebSocketMessage>>, last_ping: Arc<RwLock<DateTime<Utc>>>, out : Sender) {
    let message = *(ws_msg.read().unwrap());

    let mut last_state = message.state;

    println!("Running state change listener.");

    while !connection_expired(&last_ping){
        let message = *(ws_msg.read().unwrap());

        let current_state = message.state;

        if last_state != current_state {
            last_state = current_state;

            let msg_json = serde_json::to_string(&message).unwrap();

            let msg = format!("{}", msg_json);

            println!("Outgoing Messgae: \r\n {}", msg);

            match out.send(msg){
                Err(e) => {
                    println!("failed to send websocket message. {:?}", e);
                    break;
                },
                _ => {}
            }
        }

        thread::sleep(Duration::from_millis(500));
    }

    println!("Closing connection.");

    match out.close(CloseCode::Normal){
        _ => {}
    };
}

fn connection_expired(last_ping: &Arc<RwLock<DateTime<Utc>>>) -> bool {
    let cutoff = Utc::now() - chrono::Duration::seconds(30);

    let ping = last_ping.read().unwrap();

    match ping.cmp(&cutoff) {
        Ordering::Less => {true}
        _=> {false}
    }
}