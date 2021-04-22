use crate::models::command_models::CommandData;

use std::thread;
use std::sync::{Arc};
use std::time::Duration;

use crate::services::command_sender::CommandSender;
use crate::services::motor_message_creator::MotorMessageCreator;
use crate::services::user_service::UserService;
use crate::services::websocket_service;
use crate::services::websocket_service::{WebSocketServer, ServerState};

pub struct CommandProcessor {
    sender : CommandSender,
    user_service: UserService
}

impl CommandProcessor {
    pub fn new(
        sender : CommandSender,
        user_service : UserService
    ) -> CommandProcessor {
        CommandProcessor{
            sender,
            user_service
        }
    }

    pub fn run(&mut self) {
        let mut websocket_server = WebSocketServer::new();

        let state = Arc::clone(&websocket_server.server_state);

        // Startup Websocket server
        thread::spawn(move || {
            websocket_service::run(state);
        });

        loop {
            thread::sleep(Duration::from_secs(10));

            println!("Warning: Lock imminent");
            websocket_server.set_server_state(ServerState::Warning);

            thread::sleep(Duration::from_secs(5));

            println!("Warning: Locked");
            websocket_server.set_server_state(ServerState::Locked);

            let data = self.user_service.flush_commands();
            let num_commands = data.len();
            
            if num_commands == 0 {
                println!("No commands to process.");

                websocket_server.set_server_state(ServerState::AcceptingInput);

                continue;
            }

            println!("Found {} to process", num_commands);

            let mut command_parts : [usize; 5] = [0,0,0,0,0];

            for command in data.into_iter() {
                command_parts[0] += command.claw as usize;
                command_parts[1] += command.hand as usize;
                command_parts[2] += command.forearm as usize;
                command_parts[3] += command.strongarm as usize;
                command_parts[4] += command.shoulder as usize;
            }

            let command_parts : Vec<usize> = command_parts
                .iter()
                .map(|x| x / num_commands)
                .collect();

            println!("Averaged Commands: {:?}", &command_parts);

            let aggregate_messages = MotorMessageCreator::get_messages(CommandData{
                claw : command_parts[0] as u8,
                hand : command_parts[1] as u8,
                forearm : command_parts[2] as u8,
                strongarm : command_parts[3] as u8,
                shoulder : command_parts[4] as u8
            });

            println!("Sending commands.");

            self.sender.send_commands(aggregate_messages);

            websocket_server.set_server_state(ServerState::AcceptingInput);

            println!("Commands sent.");
        }
    }
}