use std::collections::HashMap;

/// Parse GET-parameters from url
///
/// # Examples
///
/// ```
/// let param_str = "param1=1&param2=with space&param3=3";
/// let params = ras::ras_helper::parse_get_params(param_str);

/// let vec_params = Vec::<Vec<String>>::from([
/// 	Vec::from(["param1".to_string(), "1".to_string()]),
/// 	Vec::from(["param2".to_string(), "with space".to_string()]),
/// 	Vec::from(["param3".to_string(), "3".to_string()])
/// ]);
/// assert_eq!(vec_params, params);
/// ```
pub fn parse_get_params(input_str: &str) -> Vec<Vec<String>> {
	input_str
		.split("&")
		.map(|split| split.split("=").map(|item| item.to_string()).collect())
		.collect()
}

use serde::{
	Serialize,
	Deserialize
};
/// Data for POST
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Query {
	pub token: String,
	pub data: Option<HashMap<String, String>>, 
}