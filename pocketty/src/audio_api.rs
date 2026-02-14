use crate::shared::PadId;

#[derive(Clone, Copy, Debug)]
pub struct TriggerParams {
    pub pad: PadId,
}

#[derive(Clone, Debug)]
pub enum AudioCommand {
    Trigger(TriggerParams),
}
