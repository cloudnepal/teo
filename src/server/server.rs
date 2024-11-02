use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use http_body_util::{Either, Full};
use hyper::body::{Body, Bytes, Incoming};
use hyper::header::CONTENT_TYPE;
use hyper::Method;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper_util::rt::TokioIo;
use teo_parser::ast::handler::HandlerInputFormat;
use teo_runtime::{connection};
use teo_runtime::connection::transaction;
use teo_runtime::request::Request;
use teo_runtime::response::Response;
use tokio::net::TcpListener;
use serde_json::{json, Value as JsonValue};
use teo_result::ErrorSerializable;
use teo_runtime::handler::default::{aggregate, copy, copy_many, count, create, create_many, delete, delete_many, find_first, find_many, find_unique, group_by, update, update_many, upsert};
use teo_runtime::handler::input::{validate_and_transform_json_input_for_builtin_action, validate_and_transform_json_input_for_handler};
use tower_http::services::fs::ServeFileSystemResponseBody;
use crate::app::App;
use crate::server::message::server_start_message;
use crate::prelude::Result;
use crate::prelude::Error;
use crate::server::droppable_next::DroppableNext;
use crate::server::handler_found::{find_handler, HandlerFound};
use crate::server::parse_body::{parse_form_body, parse_json_body};
use crate::server::response::hyper_response_from;
use crate::server::test_request::TestRequest;
use crate::server::test_response::TestResponse;
use crate::server::utils::remove_path_prefix;

pub struct Server {
    app: App,
}

impl Server {

    pub fn new(app: &App) -> Self {
        Self { app: app.clone() }
    }

    pub async fn before_serve(&self) -> Result<()> {
        Ok(())
    }

    pub async fn serve(&'static self, silent: bool) -> Result<()> {
        let bind = &self.app.compiled_main_namespace().server().unwrap().bind;
        let addr: SocketAddr = match format!("{}:{}", bind.0, bind.1).parse() {
            Ok(addr) => addr,
            Err(_) => return Err(Error::new(format!("cannot parse server bind address: {}:{}", bind.0, bind.1))),
        };
        let listener = TcpListener::bind(addr).await?;
        server_start_message(bind.1, &self.app.runtime_version(), &self.app.entrance(), silent)?;
        // We start a loop to continuously accept incoming connections
        loop {
            let (stream, _) = listener.accept().await?;

            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::task::spawn(async move {
                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = http1::Builder::new()
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(io, self)
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }

    async fn hyper_handler_with_error_responses(&self, hyper_request: hyper::Request<Incoming>) -> Result<hyper::Response<Either<Full<Bytes>, ServeFileSystemResponseBody>>> {
        match self.hyper_handler(hyper_request).await {
            Ok(response) => Ok(response),
            Err(error) => {
                let mut result_value = json!({
                    "type": error.inferred_title(),
                    "message": error.message(),
                });
                if error.errors.is_some() {
                    result_value["errors"] = ErrorSerializable::from_error(&error).errors;
                }
                let error_string = serde_json::to_string(&result_value).unwrap();
                Ok(hyper::Response::builder().status(error.code).header(CONTENT_TYPE, "application/json").body(Either::Left(error_string.into())).unwrap())
            },
        }
    }

    pub async fn process_request(&self, request: Request) -> Result<Response> {
        let main_namespace = self.app.compiled_main_namespace();
        let conf = self.app.compiled_main_namespace().server().unwrap();
        let droppable_next = DroppableNext::new(move |request: Request| async move {
            let path = remove_path_prefix(request.path(), conf.path_prefix());
            let Some(handler_match) = main_namespace.handler_map().match_all(request.method(), &path) else {
                return Err(Error::not_found());
            };
            request.set_handler_match(handler_match.clone());
            let Some((dest_namespace, handler_found)) = find_handler(main_namespace, &handler_match) else {
                return Err(Error::not_found());
            };
            if request.method() == Method::OPTIONS {
                return dest_namespace.handler_middleware_stack().call(request, &|_: Request| async {
                    Ok::<Response, Error>(Response::empty())
                }).await;
            }
            let incoming_string = request.take_incoming_string_for_test();
            let incoming = request.take_incoming();
            if incoming_string.is_none() && incoming.is_none() {
                return Err(Error::internal_server_error_message("HTTP body is taken"))
            }

            let body_value = if let Some(incoming) = incoming {
                match handler_found.handler_format() {
                    HandlerInputFormat::Json => if request.method() == Method::GET || request.method() == Method::DELETE {
                        JsonValue::Null
                    } else {
                        parse_json_body(incoming).await?
                    },
                    HandlerInputFormat::Form => parse_form_body(&request, incoming).await?,
                }
            } else if let Some(incoming_string) = incoming_string {
                match handler_found.handler_format() {
                    HandlerInputFormat::Json => if request.method() == Method::GET || request.method() == Method::DELETE {
                        JsonValue::Null
                    } else {
                        parse_json_body(incoming_string).await?
                    },
                    HandlerInputFormat::Form => parse_form_body(&request, incoming_string).await?,
                }
            } else {
                unreachable!()
            };
            // dispatch and run
            return match handler_found {
                HandlerFound::Builtin(model, action) => {
                    let body = validate_and_transform_json_input_for_builtin_action(model, action, &body_value, main_namespace)?;
                    request.set_body_value(body);
                    match handler_match.handler_name() {
                        "findMany" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            find_many(&request).await
                        }).await?),
                        "findFirst" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            find_first(&request).await
                        }).await?),
                        "findUnique" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            find_unique(&request).await
                        }).await?),
                        "create" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            create(&request).await
                        }).await?),
                        "delete" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            delete(&request).await
                        }).await?),
                        "update" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            update(&request).await
                        }).await?),
                        "upsert" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            upsert(&request).await
                        }).await?),
                        "copy" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            copy(&request).await
                        }).await?),
                        "createMany" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            create_many(&request).await
                        }).await?),
                        "updateMany" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            update_many(&request).await
                        }).await?),
                        "copyMany" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            copy_many(&request).await
                        }).await?),
                        "deleteMany" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            delete_many(&request).await
                        }).await?),
                        "count" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            count(&request).await
                        }).await?),
                        "aggregate" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            aggregate(&request).await
                        }).await?),
                        "groupBy" => Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, &|request: Request| async move {
                            group_by(&request).await
                        }).await?),
                        _ => Err(Error::not_found())?,
                    }
                },
                HandlerFound::Custom(handler) => {
                    let body = validate_and_transform_json_input_for_handler(handler, &body_value, main_namespace)?;
                    request.set_body_value(body);
                    Ok::<Response, Error>(dest_namespace.handler_middleware_stack().call(request, handler.call()).await?)
                }
            }
        });
        let response = main_namespace.request_middleware_stack().call(request.clone(), droppable_next.get_next()).await?;
        Ok(response)
    }

    async fn test_process(&self, test_request: TestRequest) -> Result<TestResponse> {
        let main_namespace = self.app.compiled_main_namespace();
        let conn_ctx = connection::Ctx::from_namespace(main_namespace);
        let transaction_ctx = transaction::Ctx::new(conn_ctx);
        let hyper_request = test_request.to_hyper_request();
        let request = Request::new_for_test(hyper_request, transaction_ctx);
        let response = self.process_request(request.clone()).await?;
        let hyper_response = hyper_response_from(request, response).await?;
        TestResponse::new(hyper_response).await
    }

    async fn hyper_handler(&self, hyper_request: hyper::Request<Incoming>) -> Result<hyper::Response<Either<Full<Bytes>, ServeFileSystemResponseBody>>> {
        let main_namespace = self.app.compiled_main_namespace();
        let conn_ctx = connection::Ctx::from_namespace(main_namespace);
        let transaction_ctx = transaction::Ctx::new(conn_ctx);
        let request = Request::new(hyper_request, transaction_ctx);
        let response = self.process_request(request.clone()).await?;
        hyper_response_from(request, response).await
    }
}

impl Service<hyper::Request<Incoming>> for Server {
    type Response = hyper::Response<Either<Full<Bytes>, ServeFileSystemResponseBody>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = core::result::Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: hyper::Request<Incoming>) -> Self::Future {
        let self_ = unsafe { &*(self as *const Server) } as &'static Server;
        Box::pin(self_.hyper_handler_with_error_responses(req))
    }
}