use crate::services::factory::Factory;
use crate::models::controller_models::ApiResponse;

use std::sync::Mutex;

use uuid::Uuid;

use rocket::http::Cookies;
use rocket::State;
use rocket_contrib::json;

#[post("/heartbeat", format = "application/json")]
pub fn heartbeat(cookies: Cookies, factory: State<Mutex<Factory>>) -> ApiResponse{
    let user_id = cookies.get("user_id").unwrap().value();

    let mut user_service = factory.lock().expect("Failed to obtain Factory!").user_service();

    let user_count = user_service.heartbeat_user(Uuid::parse_str(user_id).unwrap());
        
    ApiResponse{
        json: json!({"status": "success", "user_count": user_count}),
        status: rocket::http::Status::Ok
    }
}