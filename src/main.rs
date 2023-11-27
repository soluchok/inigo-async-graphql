use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    body::Body,
    response::{self, IntoResponse, Response},
    routing::get,
    Router, Server,
};

use futures_util::future::BoxFuture;
use hyper::Request;
use starwars::{QueryRoot, StarWars};
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
struct MyLayer;

impl<S> Layer<S> for MyLayer {
    type Service = MyMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyMiddleware { inner }
    }
}

#[derive(Clone)]
struct MyMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for MyMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static + Clone,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let (parts, body) = request.into_parts();
            let bytes = hyper::body::to_bytes(body).await.unwrap();
            println!("{:?}", &bytes);

            if bytes.len() > 19999990 {
                let custom_resp = String::from("{\"custom\":\"response\"}");
                return Ok(Response::builder()
                    .body(Body::from(custom_resp.to_owned()))
                    .unwrap()
                    .into_response());
            }

            let future = inner.call(Request::from_parts(parts, Body::from(bytes)));
            let response: Response = future.await?;

            let (_parts, body) = response.into_parts();
            let bytes = hyper::body::to_bytes(body).await.unwrap();
            println!("{:?}", bytes);

            Ok(Response::builder()
                .body(Body::from(bytes))
                .unwrap()
                .into_response())
        })
    }
}

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

#[tokio::main]
async fn main() {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(StarWars::new())
        .finish();

    let app = Router::new()
        .route("/", get(graphiql).post_service(GraphQL::new(schema)))
        .layer(MyLayer {});

    println!("GraphiQL IDE: http://localhost:8000");

    Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
