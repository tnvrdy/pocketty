use crate::shared::UiAction;
use crate::audio_api::{AudioCommand, TriggerParams};

pub fn action_to_audio(action: UiAction) -> Option<AudioCommand> {
    match action {
        // TODO: map PadDown to Trigger(sample_id, trim_start, length, gain, pitch, effect_chain)
        UiAction::PadDown(_pad) => None,
        _ => None,
    }
}