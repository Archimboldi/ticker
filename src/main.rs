use actix_web::{guard, web, App, HttpRequest, HttpResponse, HttpServer, Error};
use actix_web_actors::ws;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::Schema;
use async_graphql_actix_web::{Request, Response, WSSubscription};
mod books;
use books::{BooksSchema, MutationRoot, QueryRoot, SubscriptionRoot};
use sqlx::SqlitePool;
use anyhow::Result;
use dotenv::dotenv;

async fn index(schema: web::Data<BooksSchema>, req: Request) -> Response {
    schema.execute(req.into_inner()).await.into()
}

async fn index_playground() -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/").subscription_endpoint("/"),
        )))
}

async fn index_ws(
    schema: web::Data<BooksSchema>,
    req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse, Error> {
    Ok(
        ws::start_with_protocols(
            WSSubscription::new(Schema::clone(&*schema)),
            &["graphql-ws"],
            &req,
            payload,
        )?
    )
}

#[actix_rt::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set.");
    let db_pool = SqlitePool::new(&database_url).await?;

    let schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(db_pool)
        .finish();
    println!("Playground: http://localhost:8000");

    let serv = HttpServer::new(move || {
        App::new()
            .data(schema.clone())
            .service(web::resource("/").guard(guard::Post()).to(index))
            .service(
                web::resource("/")
                    .guard(guard::Get())
                    .guard(guard::Header("upgrade", "websocket"))
                    .to(index_ws),
            )
            .service(web::resource("/").guard(guard::Get()).to(index_playground))
    })
    .bind("127.0.0.1:8000")?;
    serv.run().await?;
    Ok(())
}