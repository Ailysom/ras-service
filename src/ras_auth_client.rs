use crate::{
	Verifier,
	ErrorStack
};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use reqwest::Client;

pub trait Token {
	fn get_b64(&self) -> Result<String, ()> where Self: Serialize {
		Ok(base64::encode(match serde_json::to_string(self) {
			Ok(json_str) => json_str,
			Err(err) => {
				eprintln!("Error! Can't convert token to str: {:?}", err);
				return Err(());
			}
		}))
	}
}

/// User data
///
/// User role is a bitmask, where
///
/// 0000 0001 - Service,
///
/// 0000 0010 - Administrator
///
/// **** **00 - 7 some roles
///
/// For example: Function for Administator and first role must have rule 0000 0110
#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct AccessToken {
	pub user_name: String,
	pub user_role: u8,
	pub date_spawn: u128,
}

impl AccessToken {
	pub fn new_from_str(b64_json: &str) -> Result<AccessToken, ()> {
		match serde_json::from_str(
			std::str::from_utf8(
				&base64::decode(b64_json).unwrap()
			).unwrap()
		) {
			Ok(token) => Ok(token),
			Err(err) => {
				eprintln!("Error! Can't create access token from str: {:?}", err);
				return Err(());
			},
		}
	}

	pub fn check_time(&self, token_life_time: &u128) -> bool {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or(std::time::Duration::ZERO)
			.as_millis();
		if (now - self.date_spawn) > *token_life_time {
			return false;
		} else {
			return true;
		}
	}
}

impl Token for AccessToken {}

pub trait RasAuthClient {
	/// Must return Verifier from public key
	/// For example:
	/// fn get_verifier(&self) -> std::result::Result<Verifier, ErrorStack> {
	///		Verifier::new(MessageDigest::sha256(), &self.public_key_for_token)
	/// }
	fn get_verifier(&self) 
	-> std::result::Result<Verifier, ErrorStack>;
	
	fn get_life_time_token(&self) -> u128 {
		30_000_u128
	}

	/// Check signature and life time token and return AccessToken from str token.
	fn check_and_get_access_token(&self, token_str: &str)
	-> Result<AccessToken, ()> {
		let splited_token: Vec<&str> = token_str.split("@@").collect();
		if splited_token.len() < 2 
		|| !self.check_token_sign(splited_token[0], splited_token[1]) {
			return Err(());
		}
		let token = AccessToken::new_from_str(splited_token[0])?;
		let life_time_token = self.get_life_time_token();
		if token.check_time(&life_time_token) {
			Ok(token)
		} else {
			Err(())
		}
	}

	fn check_token_sign (&self, json: &str, sign: &str) -> bool {
		let mut verifier = match self.get_verifier() {
			Ok(verifier) => verifier,
			Err(err) => {
				eprintln!("Error! Can't create verifier for token: {}", err);
				return false;
			}
		};
		match verifier.update(json.as_bytes()) {
			Ok(_) => (),
			Err(err) => {
				eprintln!("Error! Can't update data to verifier: {}", err);
				return false;
			}
		};

		match verifier.verify(&base64::decode(sign).unwrap_or(vec![0 as u8; 256])) {
			Ok(result) => result,
			Err(err) => {
				eprintln!("Error! Can't update data to verifier: {}", err);
				false
			}
		}
	}
}

/// Get public key for token from ras_auth
pub async fn get_public_key_for_token(
	login: String,
	password: String,
	ras_auth_uri: String
) -> openssl::pkey::PKey<openssl::pkey::Public> {
	let client = Client::new();
	//login
	let query = format!(r###"
		{{
			"name": "{}",
			"password": "{}"
		}}
	"###, login, password);
	let login_uri = ras_auth_uri.clone() + "/login";
	let response = client.post(login_uri)
		.body(query)
		.send().await.expect("Panic! Can't get key for token.");
	if !response.status().is_success() {
		panic!("Panic! Can't get key for token.");
	}
	let response_json = response.text().await.unwrap_or_else(|err| {
		panic!("Panic! Can't get key for token: {:?}", err);
	});
	let tokens: Value = serde_json::from_str(&response_json)
		.unwrap_or_else(|err|{
			panic!("Panic! Can't get key for token: {:?}", err);
		});
	let access_token = tokens["access_token"]
		.as_str()
		.expect("Panic! Can't get key for token: can't find access token in response");
	//get key
	let query = format!(r###"
		{{
			"token": "{}"
		}}
	"###, access_token);
	let get_public_key_uri = ras_auth_uri + "/get_public_key";
	let response = client.post(get_public_key_uri)
		.body(query)
		.send().await.expect("Panic! Can't get key for token.");
	if !response.status().is_success() {
		panic!("Panic! Can't get key for token.");
	}
	let response_json = response.text().await.unwrap_or_else(|err| {
		panic!("Panic! Can't get key for token: {:?}", err);
	});
	let pub_key: Value = serde_json::from_str(&response_json)
		.unwrap_or_else(|err|{
			panic!("Panic! Can't get key for token: {:?}", err);
		});
	openssl::pkey::PKey::public_key_from_pem(
		&base64::decode(
			pub_key["public_key"].as_str().expect("Panic! Don't exist public key")
		).unwrap_or_else(|err|{
			panic!("Panic! Can't get key for token: {:?}", err);
		})
	).unwrap_or_else(|err|{
		panic!("Panic! Can't get key for token: {:?}", err);
	})
}
