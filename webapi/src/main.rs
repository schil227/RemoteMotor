#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

mod services;
mod models;

use crate::services::motor_message_creator::MotorMessageCreator;
use crate::services::command_sender::CommandSender;
use crate::services::user_service::UserService;
use crate::services::user_service;

use std::sync::{Mutex, Arc};
use std::net::UdpSocket;

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use rocket::http::{ContentType, Status, Method, RawStr};
use rocket::request::Request;
use rocket::response;
use rocket::response::{Responder, Response};
use rocket::State;
use rocket_contrib::json;
use rocket_contrib::json::{Json, JsonValue};
use rocket_cors::{AllowedHeaders, AllowedOrigins};

// Always use a limit to prevent DoS attacks.
const LIMIT: u64 = 256;

#[derive(Debug)]
struct ApiResponse {
    json: JsonValue,
    status: Status,
}

impl<'r> Responder<'r> for ApiResponse {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        Response::build_from(self.json.respond_to(&req).unwrap())
            .status(self.status)
            .header(ContentType::JSON)
            .ok()
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct CommandData{
    claw: u8,
    hand: u8,
    forearm: u8,
    strongarm: u8,
    shoulder: u8
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/echo", format = "application/json", data= "<text>")]
fn echo(text: String) -> ApiResponse{
    println!("echo: {}", text);

    ApiResponse{
        json: json!({"status": "success", "text": text}),
        status: rocket::http::Status::Ok
    }
}

#[post("/heartbeat/<user_id>", format = "application/json")]
fn heartbeat(user_id: &RawStr, user_service: State<Arc<Mutex<UserService>>>) -> ApiResponse{

    let user_count = user_service.lock().expect("Failed to obtain command sender!")
        .heartbeat_user(Uuid::parse_str(user_id.as_str()).unwrap());
        
    ApiResponse{
        json: json!({"status": "success", "user_count": user_count}),
        status: rocket::http::Status::Ok
    }
}

#[post("/command", format = "application/json", data= "<command_data>")]
fn command(command_data: Json<CommandData>, command_sender_mutex: State<Mutex<CommandSender>>) -> ApiResponse{
    println!("Command Data: claw: {}, hand: {}, fore: {}, strong: {}, shoulder {}", 
    command_data.claw, 
    command_data.hand,
    command_data.forearm, 
    command_data.strongarm, 
    command_data.shoulder);

    let messages = MotorMessageCreator::get_messages(*command_data);

    {
        let command_sender = command_sender_mutex.lock().expect("Failed to obtain command sender!");
        
        command_sender.send_commands(messages);
    };

    ApiResponse{
        json: json!({"status": "success"}),
        status: rocket::http::Status::Ok
    }
}

fn main() {
    let client = UdpSocket::bind("192.168.1.186:7870").expect("Failed to bind client UDP socket.");

    let command_sender = Mutex::new(CommandSender::new(client, "192.168.1.38:7870".to_string()));

    let user_service = Arc::new(Mutex::new(UserService::new()));
    let user_service_reference = Arc::clone(&user_service);

    std::thread::spawn(move || {
        user_service::purge_expired_users(user_service_reference)
    });

    let allowed_origins = AllowedOrigins::All;

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::All,
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors().expect("Failed to create CORS.");

    rocket::ignite()
    .mount("/", routes![index])
    .mount("/", routes![echo])
    .mount("/", routes![command])
    .mount("/", routes![heartbeat])
    .attach(cors)
    .manage(command_sender)
    .manage(Arc::clone(&user_service))
    .launch();
}