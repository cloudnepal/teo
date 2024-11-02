#[cfg(test)]
mod tests {
    use std::cell::OnceCell;
    use teo::prelude::App;
    use std::file;
    use teo::server::server::Server;
    use teo::test::schema_path::schema_path_args;
    use serde_json::{json, Value};
    use serial_test::serial;
    use crate::{assert_json, matcher};
    use crate::lib::matcher_functions::{date_time_value, decimal_value, date_value};

    static mut SERVER: OnceCell<Server> = OnceCell::new();
    static mut BEFORE_ALL_EXECUTED: bool = false;

    fn server() -> &'static Server {
        unsafe { SERVER.get().unwrap() }
    }

    async fn before_all() {
        if unsafe { BEFORE_ALL_EXECUTED } {
            return;
        }
        unsafe {
            SERVER.get_or_init(|| {
                Server::new(App::new_with_argv(
                    schema_path_args(file!(), "schema.teo")
                ).unwrap())
            })
        };
        server().setup_app_for_unit_test().await.unwrap();
        unsafe { BEFORE_ALL_EXECUTED = true; }
    }

    async fn before_each() {
        server().reset_app_for_unit_test().await.unwrap();
    }


    #[serial]
    #[tokio::test]
    async fn int32() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "int32": 1,
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "int32": 1,
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn int64() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "int64": 1,
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "int64": 1,
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn float32() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "float32": 1.5,
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "float32": 1.5,
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn float64() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "float64": 1.2,
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "float64": 1.2,
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn bool() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "bool": true,
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "bool": true,
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn string() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "string": "KOF XV",
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "string": "KOF XV",
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn date() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "date": "2005-12-25",
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "date": date_value("2005-12-25"),
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn date_time() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "dateTime": "2003-04-17T08:12:34.567Z",
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "dateTime": date_time_value("2003-04-17T08:12:34.567Z"),
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn decimal() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "decimal": "5.78",
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "decimal": decimal_value("5.78"),
            }
        }))
    }

    #[serial]
    #[tokio::test]
    async fn r#enum() {
        before_all().await;
        before_each().await;
        let req = test::TestRequest::default()
            .method(Method::POST)
            .uri("/Support/create")
            .set_json(json!({
                "create": {
                    "sex": "FEMALE",
                },
            }))
            .to_request();
        let res: Value = test::call_and_read_body_json(&app, req).await;
        assert_json!(res, matcher!({
            "data": {
                "id": ignore,
                "sex": "FEMALE",
            }
        }))
    }
}
