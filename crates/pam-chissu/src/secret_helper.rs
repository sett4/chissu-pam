use std::env;
use std::ffi::{CString, OsString};
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
use nix::unistd::{
    fork, getegid, geteuid, initgroups, setgid, setuid, ForkResult, Gid, Pid, Uid, User,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum HelperResponse {
    Key(Vec<u8>),
    Missing { message: String },
}

#[derive(Debug)]
pub enum HelperError {
    SecretServiceUnavailable(String),
    PrivilegeDrop(PrivilegeDropFailure),
    IpcFailure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivilegeDropFailure {
    stage: PrivilegeDropStage,
    message: String,
    errno: Option<i32>,
}

impl PrivilegeDropFailure {
    pub(crate) fn new(stage: PrivilegeDropStage, message: String, errno: Option<i32>) -> Self {
        Self {
            stage,
            message,
            errno,
        }
    }

    pub fn stage(&self) -> PrivilegeDropStage {
        self.stage
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn errno(&self) -> Option<i32> {
        self.errno
    }

    pub fn is_eperm(&self) -> bool {
        self.errno == Some(libc::EPERM)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeDropStage {
    Initgroups,
    Setgid,
    Setuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrivilegeDropPlan {
    Skip,
    Switch,
}

#[derive(Debug, Clone, Default)]
pub struct HelperEnvOverrides {
    vars: Vec<(OsString, OsString)>,
}

impl HelperEnvOverrides {
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        let vars = pairs
            .into_iter()
            .map(|(k, v)| (OsString::from(k), OsString::from(v)))
            .collect();
        Self { vars }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(OsString, OsString)> {
        self.vars.iter()
    }
}

pub fn run_secret_service_helper(
    user: &str,
    timeout: Duration,
    env_overrides: Option<&HelperEnvOverrides>,
) -> Result<HelperResponse, HelperError> {
    let user_info = lookup_user(user)?;
    let (parent_stream, child_stream) =
        UnixStream::pair().map_err(|err| HelperError::IpcFailure(err.to_string()))?;

    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            drop(parent_stream);
            child_entry(user_info, child_stream, env_overrides);
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

fn child_entry(
    user: User,
    mut stream: UnixStream,
    env_overrides: Option<&HelperEnvOverrides>,
) -> ! {
    if let Some(overrides) = env_overrides {
        apply_env_overrides(overrides);
    }

    if let Err(failure) = drop_privileges(&user) {
        emit_and_exit(
            &mut stream,
            HelperWireMessage::Error {
                kind: HelperWireErrorKind::PrivilegeDrop,
                message: failure.message().into(),
                stage: Some(failure.stage()),
                errno: failure.errno(),
            },
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
                    stage: None,
                    errno: None,
                },
            );
        }
        Err(EmbeddingKeyLookupError::InvalidFormat { reason, .. }) => {
            emit_and_exit(
                &mut stream,
                HelperWireMessage::Error {
                    kind: HelperWireErrorKind::InvalidKey,
                    message: reason,
                    stage: None,
                    errno: None,
                },
            );
        }
    }
}

fn drop_privileges(user: &User) -> Result<(), PrivilegeDropFailure> {
    if privilege_drop_plan(user.uid, user.gid, geteuid(), getegid()) == PrivilegeDropPlan::Skip {
        return Ok(());
    }

    let name = CString::new(user.name.clone()).map_err(|err| {
        PrivilegeDropFailure::new(PrivilegeDropStage::Initgroups, err.to_string(), None)
    })?;
    if geteuid().is_root() {
        initgroups(&name, user.gid).map_err(|err| {
            PrivilegeDropFailure::new(
                PrivilegeDropStage::Initgroups,
                format!("privilege drop failed at initgroups: {err}"),
                Some(err as i32),
            )
        })?;
    }
    setgid(user.gid).map_err(|err| {
        PrivilegeDropFailure::new(
            PrivilegeDropStage::Setgid,
            format!("privilege drop failed at setgid: {err}"),
            (err as i32 != 0).then_some(err as i32),
        )
    })?;
    setuid(user.uid).map_err(|err| {
        PrivilegeDropFailure::new(
            PrivilegeDropStage::Setuid,
            format!("privilege drop failed at setuid: {err}"),
            (err as i32 != 0).then_some(err as i32),
        )
    })?;
    Ok(())
}

fn privilege_drop_plan(
    target_uid: Uid,
    target_gid: Gid,
    current_uid: Uid,
    current_gid: Gid,
) -> PrivilegeDropPlan {
    if current_uid == target_uid && current_gid == target_gid {
        PrivilegeDropPlan::Skip
    } else {
        PrivilegeDropPlan::Switch
    }
}

fn emit_and_exit(stream: &mut UnixStream, payload: HelperWireMessage) -> ! {
    let _ = write_payload(stream, &payload);
    let _ = stream.shutdown(Shutdown::Both);
    std::process::exit(0)
}

fn apply_env_overrides(overrides: &HelperEnvOverrides) {
    for (key, value) in overrides.iter() {
        env::set_var(key, value);
    }
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
            ..
        } => Err(HelperError::SecretServiceUnavailable(message)),
        HelperWireMessage::Error {
            kind: HelperWireErrorKind::PrivilegeDrop,
            message,
            stage,
            errno,
        } => Err(HelperError::PrivilegeDrop(PrivilegeDropFailure::new(
            stage.unwrap_or(PrivilegeDropStage::Setuid),
            message,
            errno,
        ))),
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
        #[serde(default)]
        stage: Option<PrivilegeDropStage>,
        #[serde(default)]
        errno: Option<i32>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum HelperWireErrorKind {
    SecretServiceUnavailable,
    PrivilegeDrop,
    InvalidKey,
    IpcFailure,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

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
            stage: None,
            errno: None,
        })
        .unwrap_err();
        match err {
            HelperError::SecretServiceUnavailable(message) => assert_eq!(message, "locked"),
            _ => panic!("expected secret service error"),
        }
    }

    #[test]
    fn translate_message_maps_privilege_drop_error() {
        let err = translate_message(HelperWireMessage::Error {
            kind: HelperWireErrorKind::PrivilegeDrop,
            message: "privilege drop failed at setuid: EPERM: Operation not permitted".into(),
            stage: Some(PrivilegeDropStage::Setuid),
            errno: Some(libc::EPERM),
        })
        .unwrap_err();
        match err {
            HelperError::PrivilegeDrop(failure) => {
                assert_eq!(failure.stage(), PrivilegeDropStage::Setuid);
                assert!(failure.is_eperm());
            }
            _ => panic!("expected privilege drop error"),
        }
    }

    #[test]
    fn privilege_drop_plan_skips_when_already_target_user() {
        let plan = privilege_drop_plan(
            Uid::from_raw(1000),
            Gid::from_raw(1000),
            Uid::from_raw(1000),
            Gid::from_raw(1000),
        );
        assert_eq!(plan, PrivilegeDropPlan::Skip);
    }

    #[test]
    fn privilege_drop_plan_switches_when_identity_differs() {
        let plan = privilege_drop_plan(
            Uid::from_raw(1000),
            Gid::from_raw(1000),
            Uid::from_raw(0),
            Gid::from_raw(0),
        );
        assert_eq!(plan, PrivilegeDropPlan::Switch);
    }

    #[test]
    fn helper_env_overrides_apply_variables() {
        env::remove_var("DISPLAY");
        env::remove_var("DBUS_SESSION_BUS_ADDRESS");
        let overrides = HelperEnvOverrides::from_pairs(vec![
            ("DISPLAY".into(), ":99".into()),
            (
                "DBUS_SESSION_BUS_ADDRESS".into(),
                "unix:path=/tmp/fake-bus".into(),
            ),
        ]);
        assert!(overrides.iter().next().is_some());
        apply_env_overrides(&overrides);
        assert_eq!(env::var("DISPLAY").as_deref(), Ok(":99"));
        assert_eq!(
            env::var("DBUS_SESSION_BUS_ADDRESS").as_deref(),
            Ok("unix:path=/tmp/fake-bus")
        );
    }
}
