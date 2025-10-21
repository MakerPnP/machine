use ergot::traits::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub enum Command {
    Test(u64),
}