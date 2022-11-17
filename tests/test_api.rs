use std::fs;
use std::path::Path;
use std::sync::Arc;
use rocket::local::blocking::Client;
use rocket::http::{ContentType, Status};
use jsonschema::{Draft, JSONSchema, SchemaResolverError};
use serde_json::Value;
use url::Url;

#[test]
fn can_get_index() {
    let rocket = outpack_server::api(String::from("tests/example"));
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let response = client.get("/").dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let body = serde_json::from_str(&response.into_string().unwrap()).unwrap();
    validate_success("root.json", &body);
}

#[test]
fn error_if_cant_get_index() {
    let rocket = outpack_server::api(String::from("badlocation"));
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let response = client.get("/").dispatch();

    assert_eq!(response.status(), Status::InternalServerError);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let body = serde_json::from_str(&response.into_string().unwrap()).unwrap();
    validate_error(&body, Some("No such file or directory"));
}

#[test]
fn can_get_metadata() {
    let rocket = outpack_server::api(String::from("tests/example"));
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let response = client.get("/metadata/list").dispatch();

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let body: Value = serde_json::from_str(&response.into_string().unwrap()).unwrap();
    validate_success("list.json", &body);

    let entries = body.get("data").unwrap().as_array().unwrap();
    assert_eq!(entries.len(), 3);

    assert_eq!(entries[0].get("packet").unwrap().as_str().unwrap(), "20170818-164043-7cdcde4b");
    assert_eq!(entries[0].get("time").unwrap().as_f64().unwrap(), 1662480555.6623);
    assert_eq!(entries[0].get("hash").unwrap().as_str().unwrap(),
               "sha256:1d0a4eebd63795ddff09914475efbd796defc611f7f50811284a0c01f684fa1d");

    assert_eq!(entries[1].get("packet").unwrap().as_str().unwrap(), "20170818-164830-33e0ab01");
    assert_eq!(entries[1].get("time").unwrap().as_f64().unwrap(), 1662480555.8897);
    assert_eq!(entries[1].get("hash").unwrap().as_str().unwrap(),
               "sha256:5380b3c9a1f93ab3aeaf1ed6367b98aba73dc6bfae3f68fe7d9fe05f57479cbf");
}

#[test]
fn catches_404() {
    let rocket = outpack_server::api(String::from("tests/example"));
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let response = client.get("/badurl").dispatch();

    assert_eq!(response.status(), Status::NotFound);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let body = serde_json::from_str(&response.into_string().unwrap()).unwrap();
    validate_error(&body, Some("This route does not exist"));
}

fn validate_success(schema_name: &str, instance: &Value) {
    let compiled_schema = get_schema("response-success.json");
    assert_valid(instance, &compiled_schema);
    let status = instance.get("status")
        .expect("Status property present");
    assert_eq!(status, "success");

    let data = instance.get("data")
        .expect("Data property present");
    let compiled_schema = get_schema(schema_name);
    assert_valid(data, &compiled_schema);
}

fn validate_error(instance: &Value, message: Option<&str>) {
    let compiled_schema = get_schema("response-failure.json");
    assert_valid(instance, &compiled_schema);
    let status = instance.get("status")
        .expect("Status property present");
    assert_eq!(status, "failure");

    if message.is_some() {
        let err = instance.get("errors")
            .expect("Status property present")
            .as_array().unwrap().get(0)
            .expect("First error")
            .get("detail")
            .expect("Error detail")
            .to_string();

        assert!(err.contains(message.unwrap()))
    }
}

fn assert_valid(instance: &Value, compiled: &JSONSchema) {
    let result = compiled.validate(&instance);
    if let Err(errors) = result {
        for error in errors {
            println!("Validation error: {}", error);
            println!("Instance path: {}", error.instance_path);
        }
    }
    assert!(compiled.is_valid(&instance));
}

fn get_schema(schema_name: &str) -> JSONSchema {
    let schema_path = Path::new("schema")
        .join(schema_name);
    let schema_as_string = fs::read_to_string(schema_path)
        .expect("Schema file");

    let json_schema = serde_json::from_str(&schema_as_string)
        .expect("Schema is valid json");

    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .with_resolver(LocalSchemaResolver {})
        .compile(&json_schema)
        .expect("A valid schema")
}

struct LocalSchemaResolver;

impl jsonschema::SchemaResolver for LocalSchemaResolver {
    fn resolve(&self, _root_schema: &Value, _url: &Url, original_reference: &str) -> Result<Arc<Value>, SchemaResolverError> {
        let schema_path = Path::new("schema")
            .join(original_reference);
        let schema_as_string = fs::read_to_string(schema_path)
            .expect("Schema file");
        let json_schema = serde_json::from_str(&schema_as_string)
            .expect("Schema is valid json");
        return Ok(Arc::new(json_schema));
    }
}
