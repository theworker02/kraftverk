//! Client for the privileged agent.

use kraftverk_core::error::{Error, Result};
use uuid::Uuid;

use crate::auth::load_agent_token;
use crate::protocol::{AgentRequest, AgentResponse};
use crate::transport::{self, default_endpoint, read_json, write_json};

enum Stream {
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
    #[cfg(windows)]
    Pipe(transport::win_pipe::PipeStream),
}

pub struct AgentClient {
    stream: Stream,
}

impl AgentClient {
    pub fn connect_default() -> Result<Self> {
        Self::connect(&default_endpoint())
    }

    pub fn connect(endpoint: &str) -> Result<Self> {
        let token = load_agent_token()?;
        let mut client = Self::connect_raw(endpoint)?;
        let id = Uuid::new_v4();
        let resp = client.request(AgentRequest::Auth {
            id,
            token,
            client: format!("kraftverk/{}", env!("CARGO_PKG_VERSION")),
        })?;
        match resp {
            AgentResponse::Authed { .. } => Ok(client),
            AgentResponse::Error { message, .. } => Err(Error::Platform(message)),
            _ => Err(Error::Platform("unexpected auth response".into())),
        }
    }

    fn connect_raw(endpoint: &str) -> Result<Self> {
        #[cfg(unix)]
        {
            let stream = transport::unix_sock::connect(endpoint)?;
            Ok(Self {
                stream: Stream::Unix(stream),
            })
        }
        #[cfg(windows)]
        {
            let stream = transport::win_pipe::PipeStream::connect(endpoint)?;
            Ok(Self {
                stream: Stream::Pipe(stream),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = endpoint;
            Err(Error::unsupported("agent client unsupported on this OS"))
        }
    }

    pub fn request(&mut self, req: AgentRequest) -> Result<AgentResponse> {
        match &mut self.stream {
            #[cfg(unix)]
            Stream::Unix(s) => {
                write_json(s, &req)?;
                read_json(s)
            }
            #[cfg(windows)]
            Stream::Pipe(s) => {
                write_json(s, &req)?;
                read_json(s)
            }
        }
    }

    pub fn health(&mut self) -> Result<AgentResponse> {
        self.request(AgentRequest::Health { id: Uuid::new_v4() })
    }

    pub fn ping(&mut self) -> Result<()> {
        match self.request(AgentRequest::Ping { id: Uuid::new_v4() })? {
            AgentResponse::Pong { .. } => Ok(()),
            AgentResponse::Error { message, .. } => Err(Error::Platform(message)),
            _ => Err(Error::Platform("unexpected ping response".into())),
        }
    }
}

/// Returns whether a live agent is reachable and responds to ping.
pub fn agent_connected() -> bool {
    match AgentClient::connect_default() {
        Ok(mut c) => c.ping().is_ok(),
        Err(_) => false,
    }
}

/// One-shot connect helper.
pub fn connect() -> Result<AgentClient> {
    AgentClient::connect_default()
}
