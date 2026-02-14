pub const NUM_PADS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PadId(pub u8);

#[derive(Clone, Debug)]
pub enum UiAction {
    PadDown(PadId),
    Quit,
}
