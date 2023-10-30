use crate::socket::socket_manager::SocketManager;

pub struct Forwarder<'a> {
    input: &'a SocketManager,
    output: &'a SocketManager,
}

impl<'a> Forwarder<'a> {
    pub fn new(input: &'a SocketManager, output: &'a SocketManager) -> Self {
        return Forwarder { input, output };
    }

    pub async fn start_forwarding(&self) {
        loop {
            let receive = self.input.recv().await;
            self.output.send(receive).await;
        }
    }
}
