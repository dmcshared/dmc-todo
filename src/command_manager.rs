use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CommandGroup<P, C>
where
    P: CommandPart<C>,
{
    pub commands: Vec<Command<P, C>>,
}

impl<P, C> CommandGroup<P, C>
where
    P: CommandPart<C>,
{
    pub fn to_strings(&self, context: &C) -> Vec<String> {
        let mut result = Vec::new();
        for command in &self.commands {
            result.push(command.to_string(context));
        }
        result
    }
}

#[derive(Serialize, Deserialize)]
pub struct Command<P, C>
where
    P: CommandPart<C>,
{
    pub parts: Vec<P>,
    _ctx_phantom: std::marker::PhantomData<C>,
}

impl<P, C> Command<P, C>
where
    P: CommandPart<C>,
{
    fn to_string(&self, context: &C) -> String {
        let mut result = String::new();
        for part in &self.parts {
            result.push_str(&part.to_string(context));
        }
        result
    }
}

pub trait CommandPart<C> {
    fn to_string(&self, context: &C) -> String;
}

pub enum CommandPartType {
    Direct(String),
    ConfigFile,
    ConfigFolder,
}
