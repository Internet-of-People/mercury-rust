use super::*;

//use mercury_connect::sdk::DAppApi;

//TODO 0.2
//1. make profile
//2. register profile on mercury home node
//3. pair server and client
//4. make call from client towards server to declare "active state"
//5. send event(s) from server to client(s) in active state

enum AppState{
    ServerInit(Box<Future<Item = Server, Error = AppError>>),
    ServerOpen(Box<Future<Item = ServerConnection, Error = AppError>>),
    ClientConnecting(Box<Future<Item = Client, Error = AppError>>),
    ClientConnected(Client),
}

fn state_change(state: AppState)->AppState{
    match state{
        AppState::ServerInit(fut)=>{
            fut.and_then(|_|{future::ok(())})
        },
        AppState::ServerOpen(fut)=>{
            fut.and_then(|_|{})
        },
        AppState::ClientConnecting(fut)=>{
            fut.and_then(|_|{})
        },
        AppState::ClientConnected(fut)=>{
            fut.and_then(|_|{})
        },
    }
}



struct AppError;
struct DAppApiImp;
impl DAppApiImp{
    pub fn new()-> Box< Future<Item=Self, Error=AppError> >{
        Box::new(future::ok(Self{}))
    }
}

struct ClientConnector;
impl Future for ClientConnector{
    type Item = DAppApiImp;
    type Error = AppError;

    fn poll(&mut self, cx: &mut Context) -> futures::Poll<Self::Item, Self::Error> {
        Ok(futures::Async::Ready(Self::Item::new()))
    }
}
