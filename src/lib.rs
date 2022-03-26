//! Simple library for build (non-full) REST microservice
//!
//!	Is used async tokio runtime.
//!
//! To build runtime use method "get_runtime".
//!
//! To create executor use method "new", borrowing runtime and service.
//!
//! To add functions use functions "add_get_functions" and "add_post_functions".
//!
//! Function name is last word in url.
//!
//! Signature functions:
//!
//!  fn(Handle, Arc<T>, Option<&str>) -> RasResult
//!
//! Must return RasResult::Sync for sync call,
//! and RasResult::Async for async call.
//!
//! Sync contains HttpStatus and answer data.
//! Async contains JoinHandle, wich will be awaited.
//!
//! # Examples
//!
//! ```
//! use ras_service::*;
//!
//! // Your Service (used for contains resources, as discriptos or data)
//! // Must be Sync + Send
//! struct Service {
//! 	some_data: String,
//! }
//!
//! //Build your constructor here
//! impl Service {
//! 	async fn new() -> Service {
//! 		Service {
//! 			some_data: "resource".to_string(),
//! 		}
//! 	}
//! }
//! 
//! //Sync get function
//! fn some_test_get(
//! 	runtime: Handle,
//! 	self_service: Arc<Service>,
//! 	params: Option<&str>)
//! -> RasResult {
//! 	let result = if let Some(param_str) = params {
//! 		format!(
//! 			"Your params: {:#?}",
//!				ras_service::ras_helper::parse_get_params(param_str)
//! 		)
//! 	} else {
//! 		"Empty params".to_string()
//! 	};
//! 	RasResult::Sync(
//! 		HttpStatus::OK,
//! 		Some(result)
//! 	)
//! }
//!
//! //Async post funtion
//! fn some_test_post(
//! 	runtime: Handle,
//! 	self_service: Arc<Service>,
//! 	query: Option<&str>)
//! -> RasResult {
//! 	let query: HashMap<String, Option<String>> = 
//! 		if let Some(query_str) = query {
//! 			match serde_json::from_str(query_str) {
//! 				Ok(query) => query,
//! 				Err(err) => {
//! 					eprintln!("Error! Bad json format: {:?}", err);
//! 					return RasResult::Sync(HttpStatus::BadRequest, None);
//! 				}
//! 			}
//! 		} else {
//! 			return RasResult::Sync(HttpStatus::BadRequest, None);
//! 		};
//! 	let service = self_service.clone();
//! 	RasResult::Async(runtime.spawn(async move {
//! 		let result = format!("You data: {:?}; Resource: {:?}", query, service.some_data);
//! 		(HttpStatus::OK, Some(result))
//! 	}))
//! }
//! 
//! fn main() {
//! 	let runtime = RasServiceBuilder::<Service>::get_runtime(4);
//! 	let service = runtime.block_on(async {Service::new().await});
//! 	RasServiceBuilder::new(runtime, service)
//! 		.set_socket_url("127.0.0.1:7878")
//! 		.add_get_function("some_test".to_string(), some_test_get)
//! 		.add_post_function("some_test".to_string(), some_test_post)
//! 		//.run();
//! 		;
//!	  	assert_eq!(1, 1);
//! }
//! ```

// TODO: finish the documentation
// TODO: write tests for ras auth client

/// Additional functions
pub mod ras_helper;
/// Tools for implementation of identification and authentication.
///
/// For use your service must implementation trait RasAuthClient.
/// User data contains into AccessToken.
///
/// Authentication Server - ras_auth 
#[cfg(feature = "Authentication")]
pub mod ras_auth_client;

use tokio::{
	task::JoinHandle,
	io::AsyncWriteExt
};

//re export
pub use openssl::{
	sign::Verifier,
	hash::MessageDigest,
	pkey::PKey,
	pkey::Public,
	error::ErrorStack
};
pub use tokio::runtime::Handle;
pub use std::{
	sync::{Arc, Mutex},
	collections::HashMap,
};

/// Result for user functions.
///
/// Use Async, if needed awaiting JoinHandle. 
/// In other cases use Sync
pub enum RasResult {
	Sync(HttpStatus, Option<String>),
	Async(JoinHandle<(HttpStatus, Option<String>)>),
}

/// Executor
pub struct RasServiceBuilder<T> {
	get_functions: HashMap<
		String,
		fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
	>,
	post_functions: HashMap<
		String,
		fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
	>,
	runtime: tokio::runtime::Runtime,
	service: Arc<T>,
	socket_url: String,
}

impl<T: 'static> RasServiceBuilder<T>
where T: Sync + Send {
	//methods:
	/// Generate async tokio runtime
	pub fn get_runtime(num_threads: usize) -> tokio::runtime::Runtime {
		//TODO: custom numthreads
		//      custom address:port
		tokio::runtime::Builder::new_multi_thread()
			.worker_threads(num_threads)
			.enable_io()
			.enable_time()
			.build()
			.expect("Panic! Can't build tokio runtime")
	}

	//constructor:
	/// Create executor (also bind runtime and service)
	pub fn new(runtime: tokio::runtime::Runtime, service: T)
	-> RasServiceBuilder<T>
	where T: Sync + Send {
		RasServiceBuilder {
			get_functions: HashMap::new(),
			post_functions: HashMap::new(),
			runtime: runtime,
			service: Arc::new(service),
			socket_url: "127.0.0.1:7777".to_string(),
		}
	}

	//interface:
	/// Specify address for TcpListener
	pub fn set_socket_url(
		mut self,
		url: &str,
	) -> Self {
		self.socket_url = url.to_string();
		self
	}

	/// Register GET function
	pub fn add_get_function(
		mut self,
		name: String,
		f: fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult,
	) -> Self {
		let funcs = self.mut_get_functions();
		funcs.insert(name, f);
		self
	}

	/// Register POST function
	pub fn add_post_function(
		mut self,
		name: String,
		f: fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult,
	) -> Self {
		let funcs = self.mut_post_functions();
		funcs.insert(name, f);
		self
	}

	/// Start service.
	///
	/// Accepting connections loop is running in a blocking call.
	pub fn run(self) {
		let self_arc = Arc::new(self);
		let for_start = self_arc.clone();
		for_start.runtime.block_on(async move {
			let listener = tokio::net::TcpListener::bind(&self_arc.socket_url)
				.await
				.expect("Panic! Can't bind to Tcp Sockert!");
			loop {
				let (mut stream, _addr) = match listener.accept().await {
					Ok(val) => val,
					Err(err) => {
						eprintln!("Error! Can't accept connection: {:?}", err);
						continue;
					}
				};
				let ref_service = self_arc.clone();
				tokio::spawn(async move {
					let (http_status, result_data) = 
						ref_service.connection_handler(&mut stream).await;
					ref_service
						.send_response(http_status, result_data, &mut stream).await;
				});
			}
		});
	}

	//inner functions:
	fn post_functions(&self)
	-> &HashMap<
		String,
		fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
	> {
		&self.post_functions
	}

	async fn query_handle(
		&self,
		funcs: &HashMap<
			String,
			fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
		>,
		func_name: &str,
		input_data: Option<&str>,
	) -> (HttpStatus, Option<String>) {
		let result = match funcs.get(func_name) {
			Some(func) => {
				let runtime_handler = tokio::runtime::Handle::current();
				func(runtime_handler, self.service.clone(), input_data)
			},
			None => RasResult::Sync(HttpStatus::NotFound, None),
		};
		match result {
			RasResult::Sync(http_status, data) => (http_status, data),
			RasResult::Async(join_handle) => {
				join_handle
					.await
					.unwrap_or((HttpStatus::InternalServerError, None))
			}
		}
	}

	fn get_functions(&self)
	-> &HashMap<
		String,
		fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
	> {
		&self.get_functions
	}

	fn mut_get_functions(&mut self)
	-> &mut HashMap<
		String,
		fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
	> {
		&mut self.get_functions
	}

	fn mut_post_functions(&mut self)
	-> &mut HashMap<
		String,
		fn(tokio::runtime::Handle, Arc<T>, Option<&str>) -> RasResult
	> {
		&mut self.post_functions
	}

	async fn connection_handler(
		&self,
		stream: &mut tokio::net::TcpStream,
	) -> (HttpStatus, Option<String>) {
		//TODO: take buffers sizes from config 
		const BUFFER_SIZE: usize = 2048;
		const HEADER_BUFFER_SIZE: usize = 32;

		let mut buffer = [0; BUFFER_SIZE];
		if let Err(err) = stream.readable().await {
			eprintln!("Error! Can't read data: {:?}", err);
				return (HttpStatus::InternalServerError, None);
		}
		let data_end = match stream.try_read(&mut buffer) {
			Ok(0) => {
				eprintln!("Error! Empty data");
				return (HttpStatus::BadRequest, None);
			},
			Ok(n) => n,
			Err(err) => {
				eprintln!("Error! Can't read data: {:?}", err);
				return (HttpStatus::BadRequest, None);
			}
		};
		let mut headers = [httparse::EMPTY_HEADER; HEADER_BUFFER_SIZE];
		let mut req = httparse::Request::new(&mut headers);
		let data_start = match req.parse(&buffer) {
			Ok(status) => {
				match status {
					httparse::Status::Complete(data_offset) => data_offset,
					httparse::Status::Partial => {
						return (HttpStatus::BadRequest, None)
					},
				}
			},
			Err(_) => {
				return (HttpStatus::BadRequest, None)
			},
		};
		let path = match req.path {
			Some(ref path) => path,
			None => {
				eprintln!("Error! Empty get-query path!");
				return (HttpStatus::BadRequest, None);
			}
		};
		let decode_path = urldecode::decode(path.to_string());
		let mut splited_path = match decode_path.split("/").last() {
			Some(val) => val,
			None => {
				eprintln!("Error! Bad path");
				return (HttpStatus::BadRequest, None);
			}
		}.split("?");
		let func_name = match splited_path.next() {
			Some(val) => val,
			None => {
				eprintln!("Error! Bad path to api");
				return (HttpStatus::BadRequest, None);
			}
		};	
		let params = splited_path.next();
		let (func, input_data) = match req.method.unwrap_or("") {
			"GET" => {
				(self.get_functions(), params)
				// self.get_handler(func_name, params).await
			},
			"POST" => {
				if BUFFER_SIZE < data_end 
				|| data_start > data_end {
					return (HttpStatus::BadRequest, None);
				}
				let content = match std::str::from_utf8(&buffer[data_start..data_end]) {
					Ok(content) => content,
					Err(err) => {
						eprintln!("Error! Can't convert to UTF8: {:?}", err);
						return (HttpStatus::BadRequest, None);
					},
				};
				(self.post_functions(), Some(content))
			},
			_ => return (HttpStatus::BadRequest, None),
		};
		self.query_handle(func, func_name, input_data).await
	}

	async fn send_response(
		&self,
		status_line: HttpStatus,
		content: Option<String>,
		stream: &mut tokio::net::TcpStream
	) {
		let content = content.unwrap_or("".to_string());
		let response = format!(
			"{}\r\nContent-Length: {}\r\nContent-type: application/json; charset=utf-8\r\n\r\n{}",
			status_line.get_string(),
			content.len(),
			content
		);
		match stream.write(response.as_bytes()).await {
			Ok(_) => (),
			Err(err) => {
				eprintln!("Error! Can't send data: {:?}", err);
				return;
			}
		};
		match stream.flush().await {
			Ok(_) => (),
			Err(err) => {
				eprintln!("Error! Can't send data: {:?}", err);
				return;
			}
		};		
	}
}

/// Http status for result.
#[derive(PartialEq)]
#[derive(Debug)]
pub enum HttpStatus {
	OK,
	BadRequest,
	Forbidden,
	Unauthorized,
	AuthenticationTimeout,
	InternalServerError,
	NotFound,
}

/// Get header line
impl HttpStatus {
	pub fn get_string(&self) -> String {
		match self {
			HttpStatus::OK => String::from("HTTP/1.1 200 OK"),
			HttpStatus::BadRequest => String::from("HTTP/1.1 400 Bad Request"),
			HttpStatus::Forbidden => String::from("HTTP/1.1 403 Forbidden"),
			HttpStatus::Unauthorized => String::from("HTTP/1.1 401 Unauthorized"),
			HttpStatus::AuthenticationTimeout =>
				String::from("HTTP/1.1 419 Authentication Timeout"),
			HttpStatus::NotFound => String::from("HTTP/1.1 404 Not Found"),
			HttpStatus::InternalServerError => 
				String::from("HTTP/1.1 500 InternalServerError"),
		}
	}
}

#[cfg(test)]
mod tests {
use super::*;

struct SomeService {
	}

	fn some_test_get(
		_runtime: tokio::runtime::Handle,
		_self_service: Arc<SomeService>,
		_params: Option<&str>)
	-> RasResult {
		RasResult::Sync(
			HttpStatus::OK,
			None
		)
	}

	#[test]
	fn query_handle_sync_result() {
		let runtime = RasServiceBuilder::<SomeService>::get_runtime(1);
		let service = SomeService {};
		assert_eq!({}, {});
		let rsb = RasServiceBuilder::new(runtime, service)
			.add_get_function("some_test_get".to_string(), some_test_get);
		let arc_rsb = Arc::new(rsb);
		let arc_rsb_2 = arc_rsb.clone();
		arc_rsb_2.runtime.block_on(async move {
			let func = arc_rsb.get_functions();
			let (http_status, data) = 
				arc_rsb.query_handle(&func, "some_test_get", None).await;
			assert_eq!(http_status, HttpStatus::OK);
			assert_eq!(data, None);
		});
	}
}