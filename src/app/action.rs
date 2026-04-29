use crate::app::state::State;
use anyhow::Result;

pub enum Action {}

pub fn dispatch(_state: &mut State, _action: Action) -> Result<()> {
    Ok(())
}
