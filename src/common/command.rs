use std::{
    io,
    process::{Command, Output},
};

#[mockall::automock]
pub trait CommandRunner {
    fn run(&self, command: &mut Command) -> io::Result<Output>;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, command: &mut Command) -> io::Result<Output> {
        command.output()
    }
}
