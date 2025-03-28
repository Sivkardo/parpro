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

#[derive(Debug, PartialEq, Clone, Copy)]
enum Side {
    LEFT,
    RIGHT,
}

#[derive(Debug, PartialEq)]
enum Message {
    GIVE(Side),
    REQUEST(Side),
}

impl Into<u8> for Message {
    fn into(self) -> u8 {
        match self {
            Message::GIVE(Side::RIGHT) => 0,
            Message::GIVE(Side::LEFT) => 1,
            Message::REQUEST(Side::RIGHT) => 2,
            Message::REQUEST(Side::LEFT) => 3,
        }
    }
}

impl From<u8> for Message {
    fn from(value: u8) -> Self {
        match value {
            0 => Message::GIVE(Side::RIGHT),
            1 => Message::GIVE(Side::LEFT),
            2 => Message::REQUEST(Side::RIGHT),
            3 => Message::REQUEST(Side::LEFT),
            _ => panic!("Invalid <u8> value passed as a message => {}", value)
        }
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

    fn received_fork(&mut self, msg_type: &Message, sender: i32, world: &SimpleCommunicator, indent: &String) -> Option<Side> {
        match msg_type {
            Message::GIVE(Side::LEFT) => {
                println!("{}[{}] received left fork from [{}]!", indent, world.rank(), sender);
                self.left_fork = ForkState::CLEAN;
                Some(Side::LEFT)
            } 
            Message::GIVE(Side::RIGHT) => {
                println!("{}[{}] received right fork from [{}]!", indent, world.rank(), sender);
                self.right_fork = ForkState::CLEAN;
                Some(Side::RIGHT)
            }
            _ => panic!("Invalid message received type => {:?}", msg_type)
        }
    }

    fn respond_to_msg_request(&mut self, msg_type: &Message, sender: i32, world: &SimpleCommunicator, indent: &String) {
        match msg_type {
            Message::REQUEST(Side::LEFT) => {
                if self.right_fork == ForkState::DIRTY {
                    println!("{}[{}] giving right fork to [{}]!", indent, world.rank(), sender);
                    world.process_at_rank(sender).send::<u8>(&Message::GIVE(Side::LEFT).into());
                    self.right_fork = ForkState::MISSING;
                    self.right_fork_request = false;
                } else {
                    self.right_fork_request = true
                }
            }
            Message::REQUEST(Side::RIGHT) => {
                if self.left_fork == ForkState::DIRTY {
                    println!("{}[{}] giving left fork to {}!", indent, world.rank(), sender);
                    world.process_at_rank(sender).send::<u8>(&Message::GIVE(Side::RIGHT).into());
                    self.left_fork = ForkState::MISSING;
                    self.left_fork_request = false;
                } else {
                    self.left_fork_request = true;
                }
            }
            _ => panic!("Invalid message request type => {:?}", msg_type)
        }
    }

    fn respond_to_existing_requests(&mut self, world: &SimpleCommunicator, indent: &String) {
        if self.left_fork_request {
            println!("{}[{}] sending left fork to [{}]!", indent, world.rank(), self.left_neighbour);
            world.process_at_rank(self.left_neighbour).send::<u8>(&Message::GIVE(Side::RIGHT).into());
            self.left_fork = ForkState::MISSING;
            self.left_fork_request = false;
        }
        if self.right_fork_request {
            println!("{}[{}] sending right fork to [{}]!", indent, world.rank(), self.right_neighbour);
            world.process_at_rank(self.right_neighbour).send::<u8>(&Message::GIVE(Side::LEFT).into());
            self.right_fork = ForkState::MISSING;
            self.right_fork_request = false;
        }
    }

    fn request_fork(&self, world: &SimpleCommunicator, indent: &String) -> Side {
        if self.left_fork == ForkState::MISSING {
            world.process_at_rank(self.left_neighbour).send::<u8>(&Message::REQUEST(Side::LEFT).into());
            println!("{}[{}] requested left fork from [{}]!", indent, world.rank(), self.left_neighbour);
            Side::LEFT
        } else if self.right_fork == ForkState::MISSING {
            world.process_at_rank(self.right_neighbour).send::<u8>(&Message::REQUEST(Side::RIGHT).into());
            println!("{}[{}] requested right fork from [{}]!", indent, world.rank(), self.right_neighbour);
            Side::RIGHT
        } 
        else {
            panic!("Should be unreachable!");
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
        panic!("");
    }

    let mut philosopher = Philosopher::new(size, rank);

    loop {
        let thinking_time = rand::rng().random_range(2..=5);
        println!("{}[{}] is thinking!", indent, rank);
        for _ in 0..thinking_time {
            if let Some(_) = world.any_process().immediate_probe() {
                let (msg, status)  = world.any_process().receive::<u8>();

                let msg_type = Message::from(msg);
                let sender = status.source_rank();

                match msg_type {
                    Message::GIVE(_) => {
                        philosopher.received_fork(&msg_type, sender, &world, &indent);
                    }
                    Message::REQUEST(_) => {
                        philosopher.respond_to_msg_request(&msg_type, sender, &world, &indent);
                    }
                }
            
            }
            thread::sleep(time::Duration::from_secs(1));
        } 
        println!("{}[{}] finished thinking!", indent, rank);

        while philosopher.check_forks_missing() {   

            let requested_fork = philosopher.request_fork(&world, &indent);
            let mut received = Option::None;

            while received.take() != Some(requested_fork) {
                let (msg, status) = world.any_process().receive::<u8>();

                let msg_type = Message::from(msg);
                let sender = status.source_rank();

                match msg_type {
                    Message::GIVE(_) => {
                        received = philosopher.received_fork(&msg_type, sender, &world, &indent);
                    }
                    Message::REQUEST(_) => {
                        philosopher.respond_to_msg_request(&msg_type, sender, &world, &indent);
                    }
                }
            }
        }

        println!("{}Philosopher {} is eating!", indent, rank);
        thread::sleep(time::Duration::from_secs(2));
        philosopher.eat();

        philosopher.respond_to_existing_requests(&world, &indent);
    }
}