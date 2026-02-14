use crate::shared::UiAction;
use crate::audio_api::{AudioCommand, TriggerParams};

pub fn action_to_audio(action: UiAction) -> Option<AudioCommand> {
    match action {
        UiAction::PadDown(pad) => Some(AudioCommand::Trigger(TriggerParams { pad })),
        _ => None,
    }
}