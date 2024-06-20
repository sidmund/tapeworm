use crate::types;

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Help,
    List,
    Alias,
    Show,
    Clean,
    Add,
    Download,
    Tag,
    Deposit,
    Process,
}

impl Command {
    pub fn from(s: &str) -> types::CommandResult {
        match s {
            "help" | "h" | "-h" | "--help" => Ok(Self::Help),
            "list" | "ls" | "l" => Ok(Self::List),
            "alias" => Ok(Self::Alias),
            "show" => Ok(Self::Show),
            "clean" => Ok(Self::Clean),
            "add" => Ok(Self::Add),
            "download" => Ok(Self::Download),
            "tag" => Ok(Self::Tag),
            "deposit" => Ok(Self::Deposit),
            "process" => Ok(Self::Process),
            _ => Err(format!("Unrecognized command: {}. See 'help'", s).into()),
        }
    }

    pub fn uses_lib_conf(&self) -> bool {
        match self {
            Self::Alias => true,
            Self::Clean => true,
            Self::Deposit => true,
            Self::Download => true,
            Self::Process => true,
            Self::Show => true,
            Self::Tag => true,
            _ => false,
        }
    }

    pub fn uses_cli(&self) -> bool {
        match self {
            Self::Clean => true,
            Self::Deposit => true,
            Self::Download => true,
            Self::Process => true,
            Self::Tag => true,
            _ => false,
        }
    }

    pub fn is_valid_processing_step(&self) -> bool {
        match self {
            Self::Clean => true,
            Self::Deposit => true,
            Self::Download => true,
            Self::Tag => true,
            _ => false,
        }
    }
}
