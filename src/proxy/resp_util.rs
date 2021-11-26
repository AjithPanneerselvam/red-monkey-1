use log::{debug, error};
use resp::{Decoder, Value};

use crate::proxy::connection::Error;

// Decodes the request body into Redis RESP values
//
// Returs Ok(resp::Value) on success
//
// # Errors
//
// Returns [RespErrors::DecoderFeedError] when feeding request body to the decoder fails
// or [RespErrors::DecodeError] when decoding the request body into resp::Value fails
//
// # Example
// ``` no_run
// use crate::proxy::resp_util;
//
// match resp_util::decode("i*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n") {
//      Ok(_) => {},
//      Err(e) => {
//          // handle error
//      },
//  };
// ```
pub fn decode(req_body: &str) -> Result<Value, Error> {
    let mut decoder = Decoder::new();

    match decoder.feed(req_body.as_bytes()) {
        Ok(_) => {}
        Err(err) => {
            return Err(Box::new(RespErrors::DecoderFeedError(Box::new(err))));
        }
    };

    match decoder.read() {
        Some(val) => Ok(val),
        None => Err(Box::new(RespErrors::DecodeError)),
    }
}

// Fetches the Redis command from the resp::Value::Array
//
// Returns Ok(redis_command) on success
//
// # Errors
//
// Returns [RespErrors:UnsupportedRespValError] when type other than resp::Value::Array
// is passed.
// [RespErrors::UnsupportedRespArrValError] is returned if the first value in the
// resp::Value::Array is not resp::Value::String type.
//
// This function is used in conjugation with decode() function
//
// # Example
// ``` no_run
//  match resp_util::decode("i*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n") {
//      Ok(val) => {
//          let redis_command = resp_util::fetch_redis_command(val)?;
//      },
//      Err(e) => {
//          // handle error
//      },
//  };
// ```
pub fn fetch_redis_command(resp_vals: resp::Value) -> Result<String, Error> {
    match resp_vals {
        Value::Array(arr) => {
            if arr.is_empty() {
                return Err(Box::new(RespErrors::RespArrEmptyError));
            }
            debug!("value {:?}", arr[0]);

            match arr[0].clone() {
                Value::Bulk(v) => Ok(v),
                Value::String(v) => Ok(v),
                _ => Err(Box::new(RespErrors::UnsupportedRespArrValError)),
            }
        }
        _ => Err(Box::new(RespErrors::UnsupportedRespValError)),
    }
}

// Encodes the error message into Redis RESP Error message. The RESP Error message
// follows a format like this "-Error message\r\n"
//
// Returns Ok(Vec<u8>) on success
//
// # Example
// ``` no_run
// use crate::resp_util;
//
// let encoded_error_message = resp_util::encode_error_message("Error message".to_string())?;
// ```
pub fn encode_error_message(err_message: String) -> Result<Vec<u8>, Error> {
    let err_val = Value::Error(err_message);
    Ok(err_val.encode())
}

#[derive(Debug, thiserror::Error)]
pub enum RespErrors {
    #[error("Error decoding request body to RESP values")]
    DecodeError,
    #[error("Error feeding request body to RESP decoder")]
    DecoderFeedError(Box<dyn std::error::Error>),
    #[error("Error as RESP array is empty")]
    RespArrEmptyError,
    #[error("Error as unsupported resp array value type for a redis command")]
    UnsupportedRespArrValError,
    #[error("Error as resp value type not supported for redis command")]
    UnsupportedRespValError,
}

#[cfg(test)]
mod tests {
    use crate::proxy::resp_util;

    #[test]
    fn test_decode() {
        let buf = "*3\r\n$3\r\nset\r\n$5\r\nmykey\r\n$1\r\n1\r\n";
        let res = resp_util::decode(buf);
        assert_eq!(false, res.is_err());

        let buf = "hello world; this is not a valid resp message";
        let res = resp_util::decode(buf);
        assert_eq!(true, res.is_err());
    }

    #[test]
    fn test_fetch_redis_command() {
        let buf = "*3\r\n$3\r\nset\r\n$5\r\nmykey\r\n$1\r\n1\r\n";
        let res = resp_util::decode(buf).unwrap();
        let res = resp_util::fetch_redis_command(res);
        assert_eq!(true, res.is_ok());
        assert_eq!("set", res.unwrap());

        let buf = "$-1\r\n";
        let res = resp_util::decode(buf).unwrap();
        let res = resp_util::fetch_redis_command(res);
        assert_eq!(false, res.is_ok());
    }

    #[test]
    fn test_encode_error_message() {
        let error_message = "Error message".to_string();
        let res = resp_util::encode_error_message(error_message);
        assert_eq!(true, res.is_ok());
        match res {
            Ok(v) => {
                let expected_val = "-Error message\r\n".to_string();
                let actual_val = String::from_utf8(v).unwrap();
                assert_eq!(expected_val, actual_val);
            }
            Err(_) => {}
        };
    }
}
