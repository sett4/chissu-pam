use std::ffi::CString;
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use base64::{engine::general_purpose, Engine as _};
use chissu_face_core::secret_service::{
    fetch_embedding_key, EmbeddingKeyLookupError, EmbeddingKeyStatus, AES_GCM_KEY_BYTES,
};
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, initgroups, setgid, setuid, ForkResult, Pid, User};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum HelperResponse {
    Key(Vec<u8>),
    Missing { message: String },
}

#[derive(Debug)]
pub enum HelperError {
    SecretServiceUnavailable(String),
    IpcFailure(String),
}

pub fn run_secret_service_helper(
    user: &str,
    timeout: Duration,
) -> Result<HelperResponse, HelperError> {
    let user_info = lookup_user(user)?;
    let (parent_stream, child_stream) =
        UnixStream::pair().map_err(|err| HelperError::IpcFailure(err.to_string()))?;

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            drop(parent_stream);
            child_entry(user_info, child_stream);
        }
        Ok(ForkResult::Parent { child }) => {
            drop(child_stream);
            parent_entry(parent_stream, child, timeout)
        }
        Err(err) => Err(HelperError::IpcFailure(format!("fork() failed: {err}"))),
    }
}

fn lookup_user(user: &str) -> Result<User, HelperError> {
    match User::from_name(user)
        .map_err(|err| HelperError::IpcFailure(format!("failed to resolve user '{user}': {err}")))?
    {
        Some(info) => Ok(info),
        None => Err(HelperError::IpcFailure(format!(
            "user '{user}' not found while preparing Secret Service helper"
        ))),
    }
}

fn child_entry(user: User, mut stream: UnixStream) -> ! {
    if let Err(message) = drop_privileges(&user) {
        emit_and_exit(
            &mut stream,
            HelperWireMessage::error(HelperWireErrorKind::IpcFailure, &message),
        );
    }

    let username = user.name.clone();
    match fetch_embedding_key(&username) {
        Ok(EmbeddingKeyStatus::Present(key)) => {
            let encoded = general_purpose::STANDARD.encode(key.as_bytes());
            emit_and_exit(
                &mut stream,
                HelperWireMessage::Ok {
                    embedding_key: encoded,
                },
            );
        }
        Ok(EmbeddingKeyStatus::Missing) => {
            emit_and_exit(
                &mut stream,
                HelperWireMessage::Missing {
                    message: "Embedding key not found in Secret Service.".into(),
                },
            );
        }
        Err(EmbeddingKeyLookupError::SecretService(err)) => {
            emit_and_exit(
                &mut stream,
                HelperWireMessage::Error {
                    kind: HelperWireErrorKind::SecretServiceUnavailable,
                    message: err.to_string(),
                },
            );
        }
        Err(EmbeddingKeyLookupError::InvalidFormat { reason, .. }) => {
            emit_and_exit(
                &mut stream,
                HelperWireMessage::Error {
                    kind: HelperWireErrorKind::InvalidKey,
                    message: reason,
                },
            );
        }
    }
}

fn drop_privileges(user: &User) -> Result<(), String> {
    let name = CString::new(user.name.clone())
        .map_err(|err| format!("failed to prepare username for initgroups: {err}"))?;
    initgroups(&name, user.gid).map_err(|err| err.to_string())?;
    setgid(user.gid).map_err(|err| err.to_string())?;
    setuid(user.uid).map_err(|err| err.to_string())?;
    Ok(())
}

fn emit_and_exit(stream: &mut UnixStream, payload: HelperWireMessage) -> ! {
    let _ = write_payload(stream, &payload);
    let _ = stream.shutdown(Shutdown::Both);
    std::process::exit(0)
}

fn write_payload(stream: &mut UnixStream, payload: &HelperWireMessage) -> io::Result<()> {
    let mut body = serde_json::to_vec(payload)?;
    body.push(b'\n');
    stream.write_all(&body)
}

fn parent_entry(
    mut stream: UnixStream,
    child: Pid,
    timeout: Duration,
) -> Result<HelperResponse, HelperError> {
    stream.set_read_timeout(Some(timeout)).map_err(|err| {
        HelperError::IpcFailure(format!("failed to set helper read timeout: {err}"))
    })?;

    let mut buffer = Vec::new();
    match stream.read_to_end(&mut buffer) {
        Ok(_) => {}
        Err(err)
            if err.kind() == io::ErrorKind::WouldBlock || err.kind() == io::ErrorKind::TimedOut =>
        {
            let _ = kill(child, Signal::SIGKILL);
            let _ = waitpid(child, None);
            return Err(HelperError::IpcFailure(format!(
                "Secret Service helper timed out after {timeout:?}"
            )));
        }
        Err(err) => {
            let _ = waitpid(child, None);
            return Err(HelperError::IpcFailure(format!(
                "failed to read helper response: {err}"
            )));
        }
    }

    match waitpid(child, None) {
        Ok(WaitStatus::Exited(_, 0)) => {}
        Ok(status) => {
            return Err(HelperError::IpcFailure(format!(
                "helper exited abnormally: {status:?}"
            )));
        }
        Err(err) => {
            return Err(HelperError::IpcFailure(format!("waitpid failed: {err}")));
        }
    }

    if buffer.is_empty() {
        return Err(HelperError::IpcFailure(
            "helper produced no response".into(),
        ));
    }

    let message: HelperWireMessage = serde_json::from_slice(&buffer).map_err(|err| {
        HelperError::IpcFailure(format!("failed to parse helper response: {err}"))
    })?;
    translate_message(message)
}

fn translate_message(msg: HelperWireMessage) -> Result<HelperResponse, HelperError> {
    match msg {
        HelperWireMessage::Ok { embedding_key } => {
            let decoded = general_purpose::STANDARD
                .decode(embedding_key.trim())
                .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(embedding_key.trim()))
                .map_err(|err| {
                    HelperError::IpcFailure(format!("failed to decode helper key: {err}"))
                })?;

            let key_len = decoded.len();
            if key_len != AES_GCM_KEY_BYTES {
                return Err(HelperError::IpcFailure(format!(
                    "helper returned key with {key_len} bytes"
                )));
            }

            Ok(HelperResponse::Key(decoded))
        }
        HelperWireMessage::Missing { message } => Ok(HelperResponse::Missing { message }),
        HelperWireMessage::Error {
            kind: HelperWireErrorKind::SecretServiceUnavailable,
            message,
        } => Err(HelperError::SecretServiceUnavailable(message)),
        HelperWireMessage::Error { message, .. } => Err(HelperError::IpcFailure(message)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum HelperWireMessage {
    Ok {
        #[serde(rename = "embedding_key", alias = "aes_gcm_key")]
        embedding_key: String,
    },
    Missing {
        message: String,
    },
    Error {
        kind: HelperWireErrorKind,
        message: String,
    },
}

impl HelperWireMessage {
    fn error(kind: HelperWireErrorKind, message: &str) -> Self {
        HelperWireMessage::Error {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum HelperWireErrorKind {
    SecretServiceUnavailable,
    InvalidKey,
    IpcFailure,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_message_accepts_ok_payload() {
        let payload = HelperWireMessage::Ok {
            embedding_key: general_purpose::STANDARD.encode([0x11u8; AES_GCM_KEY_BYTES]),
        };
        let response = translate_message(payload).unwrap();
        match response {
            HelperResponse::Key(bytes) => assert_eq!(bytes.len(), AES_GCM_KEY_BYTES),
            _ => panic!("unexpected helper response"),
        }
    }

    #[test]
    fn translate_message_maps_missing_to_response() {
        let response = translate_message(HelperWireMessage::Missing {
            message: "no key".into(),
        })
        .unwrap();
        match response {
            HelperResponse::Missing { message } => assert_eq!(message, "no key"),
            _ => panic!("expected missing variant"),
        }
    }

    #[test]
    fn translate_message_maps_secret_service_error() {
        let err = translate_message(HelperWireMessage::Error {
            kind: HelperWireErrorKind::SecretServiceUnavailable,
            message: "locked".into(),
        })
        .unwrap_err();
        match err {
            HelperError::SecretServiceUnavailable(message) => assert_eq!(message, "locked"),
            _ => panic!("expected secret service error"),
        }
    }
}
