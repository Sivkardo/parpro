use mpi::{topology::SimpleCommunicator, traits::*};
use rand::Rng;
use std::{ thread, time };
use std::io::{self, Write};


#[derive(Debug)]
#[derive(PartialEq)]
enum ForkState {
    MISSING,
    CLEAN,
    DIRTY,
}

impl ForkState {
    fn to_u8(&self) -> u8 {
        match self {
            ForkState::MISSING => 0,
            ForkState::CLEAN => 1,
            ForkState::DIRTY => 2,
        }
    }

    fn from_u8(val: u8) -> Self {
        match val {
            0 => ForkState::MISSING,
            1 => ForkState::CLEAN,
            2 => ForkState::DIRTY,
            _ => { panic!("Received undefined ForkState!"); }
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
    fn first_philosopher(size: i32) -> Self {
        Self {
            left_fork: ForkState::DIRTY,
            right_fork: ForkState::DIRTY,
            left_fork_request: false,
            right_fork_request: false,
            left_neighbour: 1,
            right_neighbour: size - 1,
        }
    }

    fn last_philosopher(size: i32) -> Self {
        Self {
            left_fork: ForkState::MISSING,
            right_fork: ForkState::MISSING,
            left_fork_request: false,
            right_fork_request: false,
            left_neighbour: 0,
            right_neighbour: size - 2,
        }
    }

    fn other_philosopher(rank: i32) -> Self {
        Self {
            left_fork: ForkState::CLEAN,
            right_fork: ForkState::MISSING,
            left_fork_request: false,
            right_fork_request: false,
            left_neighbour: rank + 1,
            right_neighbour: rank - 1,
        }
    }

    fn eat(&mut self) {
        self.left_fork = ForkState::DIRTY;
        self.right_fork = ForkState::DIRTY;
    }

    fn check_forks_missing(&self) -> bool {
        self.left_fork == ForkState::MISSING || self.right_fork == ForkState::MISSING
    }

    fn respond_to_requests(&mut self, world: &SimpleCommunicator, indent: &String) {
        if self.left_fork_request {
            match self.left_fork {
                ForkState::CLEAN | ForkState::MISSING => return,
                ForkState::DIRTY => {
                    self.left_fork = ForkState::CLEAN;
                    println!("{}[{}] giving left fork to [{}]!", indent, world.rank(), self.left_neighbour);
                    io::stdout().flush().unwrap();
                    world.process_at_rank(self.left_neighbour).send(&self.left_fork.to_u8());
                    self.left_fork = ForkState::MISSING;
                    self.left_fork_request = false;
                }
            }
        }

        if self.right_fork_request {
            match self.right_fork {
                ForkState::CLEAN | ForkState::MISSING => return,
                ForkState::DIRTY => {
                    self.right_fork = ForkState::CLEAN;
                    println!("{}[{}] giving right fork to [{}]!", indent, world.rank(), self.right_neighbour);
                    io::stdout().flush().unwrap();
                    world.process_at_rank(self.right_neighbour).send(&self.right_fork.to_u8());
                    self.right_fork = ForkState::MISSING;
                    self.right_fork_request = false;
                }
            }
        }
    }

    fn request_missing_fork(&self, world: &SimpleCommunicator, indent: &String) {
        if self.left_fork == ForkState::MISSING {
            println!("{}[{}] requesting left fork from [{}]!", indent, world.rank(), self.left_neighbour);
            io::stdout().flush().unwrap();
            world.process_at_rank(self.left_neighbour).send(&self.left_fork.to_u8());
        } 
        if self.right_fork == ForkState::MISSING {
            println!("{}[{}] requesting right fork from [{}]!", indent, world.rank(), self.right_neighbour);
            io::stdout().flush().unwrap();
            world.process_at_rank(self.right_neighbour).send(&self.right_fork.to_u8());
        }
    }

    fn forks_dirty(&self) -> bool {
        self.left_fork == ForkState::DIRTY || self.right_fork == ForkState::DIRTY
    }

    fn note_request(&mut self, source_rank: i32) {
        // extra check for 2 processes, left_neighbour == right_neighbour
        if source_rank == self.left_neighbour && !self.left_fork_request {
            self.left_fork_request = true;
        }
        if source_rank == self.right_neighbour {
            self.right_fork_request = true;
        }

    }

    fn received_fork(&mut self, source_rank: i32, world: &SimpleCommunicator, indent: &String) {
        // extra check for 2 processes, left_neighbour == right_neighbour
        if source_rank == self.left_neighbour && self.left_fork == ForkState::MISSING {
            self.left_fork = ForkState::CLEAN;
            println!("{}[{}] received left fork from [{}]!", indent, world.rank(), self.left_neighbour);
            io::stdout().flush().unwrap();
        }
        if source_rank == self.right_neighbour {
            self.right_fork = ForkState::CLEAN;
            println!("{}[{}] received right fork from [{}]!", indent, world.rank(), self.right_neighbour);
            io::stdout().flush().unwrap();
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

    let last = size - 1;

    let mut philosopher: Philosopher;

    match rank {
        0 => philosopher = Philosopher::first_philosopher(size),
        _ if rank == last => philosopher = Philosopher::last_philosopher(size),
        _ => philosopher = Philosopher::other_philosopher(rank),
    }

    loop {

        let thinking_time = rand::rng().random_range(3..=5);
        println!("{}[{}] is thinking!", indent, rank);

        for _ in 0..thinking_time {

            // check if philosopher received a message and respond
            if let Some(_) = world.any_process().immediate_probe() {
                let (_, status)  = world.any_process().receive::<u8>();

                philosopher.note_request(status.source_rank());
                philosopher.respond_to_requests(&world, &indent);
            }

            thread::sleep(time::Duration::from_secs(1));
        } 

        println!("{}[{}] finished thinking!", indent, rank);
        if philosopher.check_forks_missing() {
            // send requests for missing forks
            philosopher.request_missing_fork(&world, &indent);

            // check for received messages until all forks are acquired
            while philosopher.check_forks_missing() {
                // receive any message
                //println!("{}[{}] waiting for message!", indent, rank);
                let (msg, status) = world.any_process().receive::<u8>();
                //println!("{}[{}] received!", indent, rank);
                let msg_type = ForkState::from_u8(msg);

                // if message is a response to a request
                if msg_type == ForkState::CLEAN { 
                    philosopher.received_fork(status.source_rank(), &world, &indent);
                } 
                // if message is a request
                else if msg_type == ForkState::MISSING {
                    if philosopher.forks_dirty() {
                        philosopher.respond_to_requests(&world, &indent);
                    }
                    else {
                        philosopher.note_request(status.source_rank());
                    }
                }

                //println!("{}[{}] left: {:?}  right: {:?}", indent, rank, philosopher.left_fork, philosopher.right_fork);
            }
        }

        println!("{}Philosopher {} is eating!", indent, rank);
        thread::sleep(time::Duration::from_secs(2));
        philosopher.eat();

        philosopher.respond_to_requests(&world, &indent);

    }

}
