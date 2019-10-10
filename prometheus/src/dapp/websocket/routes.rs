use actix::{Actor, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse, Responder};
use actix_web_actors::ws;

pub fn init_url_mapping(service: &mut web::ServiceConfig) {
    service.service(
        web::scope("/dapp").service(web::resource("").route(web::get().to(redirect_to_ws))),
    );
}

/// Define http actor
struct DAppWebSocketServer {
    counter: u32,
}

impl Actor for DAppWebSocketServer {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<ws::Message, ws::ProtocolError> for DAppWebSocketServer {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Text(text) => ctx.text(format!("{}: {}", self.counter.to_string(), text)),
            _ => (),
        };
        self.counter += 1;
    }
}

fn redirect_to_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(DAppWebSocketServer { counter: 0 }, &req, stream);
    println!("{:?}", resp);
    resp
}
