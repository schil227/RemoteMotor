mod input_monitor;
mod message_sender;

use std::thread;
use std::sync::{Arc, Mutex};

use models::MotorCommand;
use models::MotorData;
use models::MotorMessage;

fn main() {
    let mut message = MotorMessage{
        data: MotorData{
            gpio_pin: 24,
            min: 0.02,
            max: 0.12
        },
        command: MotorCommand::Stop()
    };

    let command = Arc::new(Mutex::new(message));
    let other_command = Arc::clone(&command);
    
    let message_sender_handler = thread::spawn(move || message_sender::send_command(Arc::clone(&command)));

    let input_monitor_handler = thread::spawn(move ||input_monitor::listen_for_command(Arc::clone(&other_command)));

    println!("Running.");

    message_sender_handler.join().unwrap();
    input_monitor_handler.join().unwrap();
}