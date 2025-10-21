use ergot::traits::Schema;
use serde::{Deserialize, Serialize};

#[derive(Schema, Debug, PartialEq, Serialize, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Command {
    Test(u64),
    BeginYeetTest,
    EndYeetTest,
}