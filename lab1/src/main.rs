use mpi::{topology::SimpleCommunicator, traits::*};
use rand::Rng;
use core::panic;
use std::{ thread, time };

#[derive(Debug)]
#[derive(PartialEq)]
enum ForkState {
    MISSING,
    CLEAN,
    DIRTY,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Side {
    LEFT,
    RIGHT,
    NONE,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Message {
    GiveRightFork,
    GiveLeftFork,
    RequestRightFork,
    RequestLeftFork,
}

impl Message {
    fn to_u8(&self) -> u8 {
        match self {
            Message::GiveRightFork => 0,
            Message::GiveLeftFork => 1,
            Message::RequestRightFork => 2,
            Message::RequestLeftFork => 3,
        }
    }

    fn from_u8(val: u8) -> Self {
        match val {
            0 => Message::GiveRightFork,
            1 => Message::GiveLeftFork,
            2 => Message::RequestRightFork,
            3 => Message::RequestLeftFork,
            _ => { panic!("Received undefined Message!"); }
        }
    }

    fn is_request_msg(&self) -> bool {
        *self == Message::RequestLeftFork || *self == Message::RequestRightFork
    }

    fn is_give_msg(&self) -> bool {
        *self == Message::GiveLeftFork || *self == Message::GiveRightFork
    }
}



#[derive(Debug)]
struct Philosopher {
    left_fork: ForkState,
    right_fork: ForkState,
    left_fork_request: bool,
    right_fork_request: bool,
    left_neighbour: i32,
    right_neighbour: i32,
}

impl Philosopher {

    fn new(size: i32, rank: i32) -> Self {
        if rank == 0 {
            Self {
                left_fork: ForkState::DIRTY,
                right_fork: ForkState::DIRTY,
                left_fork_request: false,
                right_fork_request: false,
                left_neighbour: 1,
                right_neighbour: size - 1,
            }
        } else if rank == size - 1 {
            Self {
                left_fork: ForkState::MISSING,
                right_fork: ForkState::MISSING,
                left_fork_request: false,
                right_fork_request: false,
                left_neighbour: 0,
                right_neighbour: size - 2,
            }
        } else {
            Self {
                left_fork: ForkState::DIRTY,
                right_fork: ForkState::MISSING,
                left_fork_request: false,
                right_fork_request: false,
                left_neighbour: rank + 1,
                right_neighbour: rank - 1,
            }
        }
    }

    fn eat(&mut self) {
        self.left_fork = ForkState::DIRTY;
        self.right_fork = ForkState::DIRTY;
    }

    fn check_forks_missing(&self) -> bool {
        self.left_fork == ForkState::MISSING || self.right_fork == ForkState::MISSING
    }

    fn received_fork(&mut self, msg_type: &Message, sender: i32, world: &SimpleCommunicator, indent: &String) -> Side {
        if *msg_type == Message::GiveLeftFork {
            println!("{}[{}] received left fork from [{}]!", indent, world.rank(), sender);
            self.left_fork = ForkState::CLEAN;
            Side::LEFT
        } 
        else if *msg_type == Message::GiveRightFork {
            println!("{}[{}] received right fork from [{}]!", indent, world.rank(), sender);
            self.right_fork = ForkState::CLEAN;
            Side::RIGHT
        }
        else {
            panic!("")
        }
    }

    fn respond_to_msg_request(&mut self, msg_type: &Message, sender: i32, world: &SimpleCommunicator, indent: &String) {
        if *msg_type == Message::RequestLeftFork {
            if self.right_fork == ForkState::DIRTY {
                println!("{}[{}] giving right fork to [{}]!", indent, world.rank(), sender);
                world.process_at_rank(sender).send(&Message::GiveLeftFork.to_u8());
                self.right_fork = ForkState::MISSING;
                self.right_fork_request = false;
            } else {
                self.right_fork_request = true
            }
        } 
        else if *msg_type == Message::RequestRightFork {
            if self.left_fork == ForkState::DIRTY {
                println!("{}[{}] giving left fork to {}!", indent, world.rank(), sender);
                world.process_at_rank(sender).send(&Message::GiveRightFork.to_u8());
                self.left_fork = ForkState::MISSING;
                self.left_fork_request = false;
            } else {
                self.left_fork_request = true;
            }
        }
    }

    fn respond_to_existing_requests(&mut self, world: &SimpleCommunicator, indent: &String) {
        if self.left_fork_request {
            println!("{}[{}] sending left fork to [{}]!", indent, world.rank(), self.left_neighbour);
            world.process_at_rank(self.left_neighbour).send(&Message::GiveRightFork.to_u8());
            self.left_fork = ForkState::MISSING;
            self.left_fork_request = false;
        }
        if self.right_fork_request {
            println!("{}[{}] sending right fork to [{}]!", indent, world.rank(), self.right_neighbour);
            world.process_at_rank(self.right_neighbour).send(&Message::GiveLeftFork.to_u8());
            self.right_fork = ForkState::MISSING;
            self.right_fork_request = false;
        }
    }

    fn request_fork(&self, world: &SimpleCommunicator, indent: &String) -> Side {
        if self.left_fork == ForkState::MISSING {
            world.process_at_rank(self.left_neighbour).send(&Message::RequestLeftFork.to_u8());
            println!("{}[{}] requested left fork from [{}]!", indent, world.rank(), self.left_neighbour);
            Side::LEFT
        } else if self.right_fork == ForkState::MISSING {
            world.process_at_rank(self.right_neighbour).send(&Message::RequestRightFork.to_u8());
            println!("{}[{}] requested right fork from [{}]!", indent, world.rank(), self.right_neighbour);
            Side::RIGHT
        } 
        else {
            panic!("");
        }
    }
    
}
    


fn main() {

    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let size = world.size();
    let rank = world.rank();

    let indent = "      ".repeat(rank.try_into().unwrap());

    if size < 2 {
        panic!("TODO");
    }

    let mut philosopher = Philosopher::new(size, rank);

    loop {
        //----------------- THINKING BLOCK START-----------------//
        let thinking_time = rand::rng().random_range(2..=5);
        println!("{}[{}] is thinking!", indent, rank);
        for _ in 0..thinking_time {
            // check if philosopher received a message and respond
            if let Some(_) = world.any_process().immediate_probe() {
                let (msg, status)  = world.any_process().receive::<u8>();

                let msg_type = Message::from_u8(msg);
                let sender = status.source_rank();

                //----------------- RECEIVED MESSAGE-----------------//
                if msg_type.is_give_msg() {
                    philosopher.received_fork(&msg_type, sender, &world, &indent);
                }
                
                //----------------- RESPOND TO A REQUEST-----------------//
                if msg_type.is_request_msg() {
                    philosopher.respond_to_msg_request(&msg_type, sender, &world, &indent);
                }
            
            }
            thread::sleep(time::Duration::from_secs(1));
        } 
        println!("{}[{}] finished thinking!", indent, rank);
        //----------------- THINKING BLOCK END-----------------//


        //----------------- REQUESTING BLOCK START-----------------//
        while philosopher.check_forks_missing() {   

            let requested_fork = philosopher.request_fork(&world, &indent);

            // Side::NONE might be redundant?
            let mut received = Side::NONE;

            
            // Wait until you receive the requested fork, save any incoming fork requests
            while received != requested_fork {

                // Receive message from any source
                //println!("{}[{}] {:?}", indent, rank, philosopher);
                let (fork, status) = world.any_process().receive::<u8>();

                let msg_type = Message::from_u8(fork);
                let sender = status.source_rank();

                //----------------- RECEIVED MESSAGE-----------------//
                if msg_type.is_give_msg() {
                    received = philosopher.received_fork(&msg_type, sender, &world, &indent);
                }
                
                //----------------- RESPOND TO A MESSAGE REQUEST-----------------//
                if msg_type.is_request_msg() {
                    philosopher.respond_to_msg_request(&msg_type, sender, &world, &indent);
                }
            }
            
        }
        //----------------- REQUESTING BLOCK END-----------------//

        //----------------- EATING BLOCK START-----------------//
        println!("{}Philosopher {} is eating!", indent, rank);
        thread::sleep(time::Duration::from_secs(2));
        philosopher.eat();
        //----------------- EATING BLOCK END-----------------//

        //----------------- RESPOND TO EXISTING REQUESTS -----------------//
        philosopher.respond_to_existing_requests(&world, &indent);
    }

}