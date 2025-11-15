use thiserror::Error;
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::OwnedObjectPath,
};

#[derive(Debug, Default, Clone)]
pub struct LogindInspector;

impl LogindInspector {
    pub fn new() -> Self {
        Self
    }

    pub fn inspect(
        &self,
        uid: u32,
        tty_hint: Option<&str>,
    ) -> Result<Option<SessionEnvironment>, LogindInspectorError> {
        let connection = Connection::system()?;
        let manager = Proxy::new(
            &connection,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )?;
        let user_path: OwnedObjectPath = manager.call("GetUser", &(uid,))?;
        let user_proxy = Proxy::new(
            &connection,
            "org.freedesktop.login1",
            user_path.as_ref(),
            "org.freedesktop.login1.User",
        )?;

        let runtime_dir = normalize_string(user_proxy.get_property::<String>("RuntimePath").ok());
        let sessions: Vec<(String, OwnedObjectPath)> = user_proxy.get_property("Sessions")?;
        if sessions.is_empty() {
            return Ok(None);
        }

        let mut records = Vec::with_capacity(sessions.len());
        for (session_id, path) in sessions {
            let proxy = Proxy::new(
                &connection,
                "org.freedesktop.login1",
                path.as_ref(),
                "org.freedesktop.login1.Session",
            )?;
            let mut record = SessionRecord {
                id: session_id,
                seat: proxy
                    .get_property::<(String, OwnedObjectPath)>("Seat")
                    .ok()
                    .and_then(|(seat, _)| normalize_string(Some(seat))),
                tty: proxy
                    .get_property::<String>("TTY")
                    .ok()
                    .and_then(|tty| normalize_tty(&tty)),
                state: proxy
                    .get_property::<String>("State")
                    .ok()
                    .and_then(|s| normalize_string(Some(s)))
                    .unwrap_or_else(|| "unknown".into()),
                class: proxy
                    .get_property::<String>("Class")
                    .ok()
                    .and_then(|s| normalize_string(Some(s)))
                    .unwrap_or_else(|| "unknown".into()),
                display: proxy
                    .get_property::<String>("Display")
                    .ok()
                    .and_then(|s| normalize_string(Some(s))),
                session_type: proxy
                    .get_property::<String>("Type")
                    .ok()
                    .and_then(|s| normalize_string(Some(s))),
                active: false,
            };
            if record.state.is_empty() {
                record.state = "unknown".into();
            }
            if record.class.is_empty() {
                record.class = "unknown".into();
            }
            record.active = proxy.get_property::<bool>("Active").unwrap_or(false);
            records.push(record);
        }

        let selection = select_session(&records, tty_hint);
        let Some(record) = selection.cloned() else {
            return Ok(None);
        };

        Ok(Some(SessionEnvironment::from_record(record, runtime_dir)))
    }
}

#[derive(Debug, Error)]
pub enum LogindInspectorError {
    #[error("logind D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),
}

#[derive(Debug, Clone)]
struct SessionRecord {
    id: String,
    seat: Option<String>,
    tty: Option<String>,
    state: String,
    class: String,
    active: bool,
    display: Option<String>,
    session_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionEnvironment {
    pub session_id: String,
    pub seat: Option<String>,
    pub tty: Option<String>,
    pub session_type: Option<String>,
    pub display: Option<String>,
    pub runtime_dir: Option<String>,
    pub dbus_address: Option<String>,
}

impl SessionEnvironment {
    fn from_record(record: SessionRecord, runtime_dir: Option<String>) -> Self {
        let dbus_address = runtime_dir
            .as_ref()
            .filter(|dir| !dir.is_empty())
            .map(|dir| format!("unix:path={}/bus", dir.trim_end_matches('/')));
        Self {
            session_id: record.id,
            seat: record.seat,
            tty: record.tty,
            session_type: record.session_type,
            display: record.display,
            runtime_dir,
            dbus_address,
        }
    }

    pub fn env_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(display) = &self.display {
            pairs.push(("DISPLAY".into(), display.clone()));
            if self.session_type.as_deref() == Some("wayland") {
                pairs.push(("WAYLAND_DISPLAY".into(), display.clone()));
            }
        }
        if let Some(runtime) = &self.runtime_dir {
            pairs.push(("XDG_RUNTIME_DIR".into(), runtime.clone()));
        }
        if let Some(address) = &self.dbus_address {
            pairs.push(("DBUS_SESSION_BUS_ADDRESS".into(), address.clone()));
        }
        pairs
    }

    pub fn summary(&self) -> String {
        format!(
            "session={} tty={} seat={} type={} display={} runtime={}",
            &self.session_id,
            self.tty.as_deref().unwrap_or("-"),
            self.seat.as_deref().unwrap_or("-"),
            self.session_type.as_deref().unwrap_or("-"),
            self.display.as_deref().unwrap_or("-"),
            self.runtime_dir.as_deref().unwrap_or("-"),
        )
    }
}

fn select_session<'a>(
    sessions: &'a [SessionRecord],
    tty_hint: Option<&str>,
) -> Option<&'a SessionRecord> {
    let normalized_hint = tty_hint.and_then(normalize_tty);
    if let Some(hint) = normalized_hint.as_deref() {
        if let Some(session) = sessions
            .iter()
            .find(|s| s.active && s.class == "user" && s.tty.as_deref() == Some(hint))
        {
            return Some(session);
        }
    }

    if let Some(session) = sessions.iter().find(|s| s.active && s.class == "user") {
        return Some(session);
    }

    sessions
        .iter()
        .find(|s| s.class == "user" && s.state == "active")
        .or_else(|| sessions.iter().find(|s| s.class == "user"))
        .or_else(|| sessions.first())
}

fn normalize_tty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.strip_prefix("/dev/").unwrap_or(trimmed).to_string();
    Some(normalized)
}

fn normalize_string(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, active: bool, class: &str, tty: Option<&str>) -> SessionRecord {
        SessionRecord {
            id: id.into(),
            seat: Some("seat0".into()),
            tty: tty.map(|v| v.into()),
            state: if active { "active" } else { "closing" }.into(),
            class: class.into(),
            active,
            display: Some(":0".into()),
            session_type: Some("x11".into()),
        }
    }

    #[test]
    fn select_prefers_matching_tty() {
        let sessions = vec![
            record("2", true, "user", Some("tty3")),
            record("3", true, "user", Some("tty2")),
        ];
        let selected = select_session(&sessions, Some("/dev/tty2")).unwrap();
        assert_eq!(selected.id, "3");
    }

    #[test]
    fn select_falls_back_to_active_user() {
        let sessions = vec![record("2", true, "user", Some("tty3"))];
        let selected = select_session(&sessions, None).unwrap();
        assert_eq!(selected.id, "2");
    }

    #[test]
    fn env_pairs_include_bus_and_wayland() {
        let record = SessionRecord {
            id: "5".into(),
            seat: None,
            tty: Some("tty2".into()),
            state: "active".into(),
            class: "user".into(),
            active: true,
            display: Some("wayland-0".into()),
            session_type: Some("wayland".into()),
        };
        let env = SessionEnvironment::from_record(record, Some("/run/user/1000".into()));
        let mut pairs = env.env_pairs();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(pairs.len(), 4);
        assert_eq!(
            pairs[0],
            (
                "DBUS_SESSION_BUS_ADDRESS".into(),
                "unix:path=/run/user/1000/bus".into()
            )
        );
        assert_eq!(pairs[1], ("DISPLAY".into(), "wayland-0".into()));
        assert_eq!(pairs[2], ("WAYLAND_DISPLAY".into(), "wayland-0".into()));
        assert_eq!(
            pairs[3],
            ("XDG_RUNTIME_DIR".into(), "/run/user/1000".into())
        );
    }
}
