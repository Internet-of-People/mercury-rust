use claims::model::TimeStamp;
use did::model::ProfileId;
use mercury_home_protocol::AsyncFallible;

pub type MessageContent = String;

pub struct Message {
    pub message: MessageContent,
    pub sender: ProfileId,
    pub receiver: ProfileId,
    pub timestamp: TimeStamp,
}

pub trait MessageApi {
    fn send_message(&self, to: &ProfileId, message: &MessageContent) -> AsyncFallible<()>;
    fn list_messages(&self, with: &ProfileId) -> AsyncFallible<Vec<Message>>;
}

pub struct MessagingImpl {
    // ???
}

impl MessageApi for MessagingImpl {
    fn send_message(&self, _to: &ProfileId, _message: &MessageContent) -> AsyncFallible<()> {
        // - load to_profile from public DHT-liek storage
        // - connect home of that profile
        // - send message via home to target profile
        unimplemented!()
    }

    fn list_messages(&self, _with: &ProfileId) -> AsyncFallible<Vec<Message>> {
        unimplemented!()
    }
}
