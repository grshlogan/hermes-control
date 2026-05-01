use hermes_control_types::RequesterChannel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiBoundary {
    pub channel: RequesterChannel,
    pub raw_process_execution: bool,
}

pub fn gui_boundary() -> GuiBoundary {
    GuiBoundary {
        channel: RequesterChannel::Gui,
        raw_process_execution: false,
    }
}
