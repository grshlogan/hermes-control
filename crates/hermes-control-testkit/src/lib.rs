use hermes_control_types::{Requester, RequesterChannel};

pub fn telegram_requester(user_id: impl Into<String>, chat_id: impl Into<String>) -> Requester {
    Requester {
        channel: RequesterChannel::Telegram,
        user_id: user_id.into(),
        chat_id: Some(chat_id.into()),
    }
}
