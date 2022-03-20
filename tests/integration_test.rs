use ras_service::{
	RasServiceBuilder,
	HttpStatus,
	ras_helper::Query,
	RasResult
};
use std::{
	sync::Arc
};
use reqwest::blocking::Client;

struct Service {
	some_data: String,
}

impl Service {
	async fn new() -> Service {
		Service {
			some_data: "resource".to_string(),
		}
	}
}

fn some_test_post(
	runtime: tokio::runtime::Handle,
	self_service: Arc<Service>,
	query: Option<&str>)
-> RasResult {
	let query: Query = if let Some(query_str) = query {
		match serde_json::from_str(query_str) {
			Ok(query) => query,
			Err(err) => {
				eprintln!("Error! Bad json format: {:?}", err);
				return RasResult::Sync(HttpStatus::BadRequest, None);
			}
		}
	} else {
		return RasResult::Sync(HttpStatus::BadRequest, None);
	};
	let service = self_service.clone();
	RasResult::Async(runtime.spawn(async move {
		let result = format!("You data: {:?}; Resource: {:?}", query, service.some_data);
		(HttpStatus::OK, Some(result))
	}))
}

fn some_test_get(
	_runtime: tokio::runtime::Handle,
	_self_service: Arc<Service>,
	params: Option<&str>)
-> RasResult {
	let result = if let Some(param_str) = params {
		format!(
			"Your params: {:?}",
			ras_service::ras_helper::parse_get_params(param_str)
		)
	} else {
		"Empty params".to_string()
	};
	RasResult::Sync(
		HttpStatus::OK,
		Some(result)
	)
}

#[test]
fn main_integraion_test() {
	let runtime = RasServiceBuilder::<Service>::get_runtime(4);
	let service = runtime.block_on(async {Service::new().await});
	let rsb = RasServiceBuilder::new(runtime, service)
		.set_socket_url("127.0.0.1:7878")
		.add_get_function("some_test".to_string(), some_test_get)
		.add_post_function("some_test".to_string(), some_test_post);
	std::thread::spawn(move || {
		rsb.run();
	});
	let join_handle_client = std::thread::spawn(||{
		std::thread::sleep(std::time::Duration::from_secs(4));
		let client = Client::new();
		let res = client.post("http://127.0.0.1:7878/api/some_test").body("
		{
			\"token\":\"some_token\",
			\"data\":
				{
					\"param1\":\"p1\"
				}
		}
		").send().unwrap();
		assert_eq!(reqwest::StatusCode::OK, res.status());
		let result = "You data: Query { token: \"some_token\", data: Some({\"param1\": \"p1\"}) }; Resource: \"resource\""
			.to_string();
		assert_eq!(result, res.text().unwrap());
		let res = client.get(
			"http://127.0.0.1:7878/api/some_test?param1=hello&parram2=world"
		).send().unwrap();		
		assert_eq!(reqwest::StatusCode::OK, res.status());
		let result = "Your params: [[\"param1\", \"hello\"], [\"parram2\", \"world\"]]"
			.to_string();
		assert_eq!(result, res.text().unwrap());
	});
	join_handle_client.join().unwrap();
}