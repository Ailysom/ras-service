use std::collections::HashMap;

/// Parse GET-parameters from url
///
/// # Examples
///
/// ```
/// let param_str = "param1=1&param2=with space&param3=";
/// let params = ras_service::ras_helper::parse_get_params(param_str);

/// let mut heshmap_params = std::collections::HashMap::new();
/// heshmap_params.insert("param1".to_string(), Some("1".to_string()));
/// heshmap_params.insert("param2".to_string(), Some("with space".to_string()));
/// heshmap_params.insert("param3".to_string(), None);
/// assert_eq!(heshmap_params, params);
/// ```
pub fn parse_get_params(input_str: &str) -> HashMap<String, Option<String>> {
	let mut result = HashMap::new();
	for line in input_str.split("&") {
		let mut param = line.split("=");
		let key = match param.next() {
			Some(val) => val.to_string(),
			_ => continue,
		};
		let value = match param.next() {
			Some(val) => match val {
					"" => None,
					_ => Some(val.to_string())
				},
			_ => None
		};
		result.insert(key, value);
	}
	result
}