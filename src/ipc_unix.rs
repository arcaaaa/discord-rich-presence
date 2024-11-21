use crate::discord_ipc::DiscordIpc;
use serde_json::json;
use std::os::unix::net::UnixStream;
use std::{
    env::var,
    io::{Read, Write},
    net::Shutdown,
    path::PathBuf,
};
use crate::Error;

// Environment keys to search for the Discord pipe
const ENV_KEYS: [&str; 4] = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"];

const APP_SUBPATHS: [&str; 4] = [
    "",
    "app/com.discordapp.Discord/",
    "snap.discord-canary/",
    "snap.discord/",
];

type Result<T> = std::result::Result<T, Error>;

#[allow(dead_code)]
#[derive(Debug)]
/// A wrapper struct for the functionality contained in the
/// underlying [`DiscordIpc`](trait@DiscordIpc) trait.
pub struct DiscordIpcClient {
    /// Client ID of the IPC client.
    pub client_id: String,
    connected: bool,
    socket: Option<UnixStream>,
}

impl DiscordIpcClient {
    /// Creates a new `DiscordIpcClient`.
    ///
    /// # Examples
    /// ```
    /// let ipc_client = DiscordIpcClient::new("<some client id>")?;
    /// ```
    pub fn new(client_id: &str) -> Result<Self> {
        let client = Self {
            client_id: client_id.to_string(),
            connected: false,
            socket: None,
        };

        Ok(client)
    }

    fn get_pipe_pattern() -> PathBuf {
        println!("{}", var("SNAP").is_ok());
        let mut path = String::new();

        for key in &ENV_KEYS {
            match var(key) {
                Ok(val) => {
                    if var("SNAP").is_ok() {
                        if key == &ENV_KEYS[0] {
                            path = val.rsplit_once('/').map(|(parent, _)| parent)
                            .unwrap_or("").to_string();
                        } 
                    }
                    else {
                        path = val;
                    }
                    break;
                }
                Err(_e) => continue,
            }
        }
        PathBuf::from(path)
    }
}

impl DiscordIpc for DiscordIpcClient {
    fn connect_ipc(&mut self) -> Result<()> {
        for i in 0..10 {
            for subpath in APP_SUBPATHS {
                let path = DiscordIpcClient::get_pipe_pattern()
                    .join(subpath)
                    .join(format!("discord-ipc-{}", i));

                println!("{}", path.display());

                match UnixStream::connect(&path) {
                    Ok(socket) => {
                        self.socket = Some(socket);
                        return Ok(());
                    }
                    Err(err) => {
                        print!("{} ", err);
                        continue;
                    },
                }
            }
        }

        // Err("Couldn't connect to the Discord IPC socket".into())
        Err(Error::IPCConnectionFailled)

    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(Error::NotConnected)?;

        socket.write_all(data)?;

        Ok(())
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<()> {
        let socket = self.socket.as_mut().ok_or(Error::NotConnected)?;

        socket.read_exact(buffer)?;

        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        let data = json!({});
        if self.send(data, 2).is_ok() {} // ?
      
        let socket = self.socket.as_mut().ok_or(Error::NotConnected)?;

        socket.flush()?;
        match socket.shutdown(Shutdown::Both) {
            Ok(()) => (),
            Err(_err) => (),
        };

        Ok(())
    }

    fn get_client_id(&self) -> &String {
        &self.client_id
    }
}
