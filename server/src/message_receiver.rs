use super::motor_controller::MotorControlData;
use super::controller_master::ControllerMaster;

use std::time::Duration;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use models::MotorMessage;
use models::MotorCommand;
use models::MotorName;

const SIZE : usize = std::mem::size_of::<MotorMessage>();

pub fn listen(){
    let listener = TcpListener::bind("192.168.1.38.7870").expect("Failed to bind listening TCP socket.");

    let mut master = ControllerMaster::new();

    loop{
        match listener.accept() {
            Ok((mut stream, addr)) => {
                println!("client: {:?}", addr);

                stream.set_read_timeout(Some(Duration::from_secs(3))).expect("Failed to set read timeout.");

                let mut num_commands = [0;1];
                stream.read(&mut num_commands).expect("Could not read count!");

                let num_commands: u8 = bincode::deserialize(&num_commands).expect("Failed to deserialize commands.");
                println!("count: {:?}", num_commands);

                send_ack(&mut stream);

                let mut messages: Vec<MotorMessage> = Vec::new();

                for _ in 0..num_commands{
                    let mut buf = [0; SIZE + 10];

                    stream.read(&mut buf).expect("Failed receiving message.");
            
                    let message : MotorMessage = bincode::deserialize(&buf).expect("Could not deserialize MotorDirection!");

                    messages.push(message);

                    send_ack(&mut stream);
                }

                println!("Received {} messages: {:?}", messages.len(), messages);

                process_messages(messages, &mut master);
            },
            Err(e) => println!("Failed to get TCP client: {:?}", e)
        }
    }
}

fn send_ack(stream : &mut TcpStream) {
    let ack : u8 = 1;
    stream.write(&ack.to_be_bytes()).expect("Failed to send ACK.");
}

fn process_messages(messages: Vec<MotorMessage>, master: &mut ControllerMaster){
    for message in messages{
        // empty pin means to stop everything
        if message.data.motor_name == MotorName::ALL {
            master.command_all_controllers(MotorCommand::Stop());
            return;
        }

        match master.get_controller(message.data.motor_name){
            Some(controller) => {controller.update(message.command)},
            None => {
                match MotorControlData::register(message.data, &master.pwm_handle){
                    Ok(controller) => {
                        controller.update(message.command);
                        master.add_controller(message.data.motor_name, controller);
                    },
                    Err(error) => { println!("{}",error); }
                };
            }
        }
    }
}