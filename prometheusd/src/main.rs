use actix_web::{web, App, HttpServer, Responder};

fn index(info: web::Path<(u32, String)>) -> impl Responder {
    format!("Hello {}! id:{}", info.1, info.0)
}

fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(web::resource("/{id}/{name}/index.html").to(index)))
        .bind("127.0.0.1:8080")?
        .run()
}
