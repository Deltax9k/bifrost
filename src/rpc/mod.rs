#[macro_use]
pub mod proto;

use std::collections::HashMap;
use std::sync::Arc;
use std::io;
use std::time::Duration;
use parking_lot::{Mutex, RwLock};
use std::thread;
use tcp;
use utils::time;
use utils::u8vec::*;
use futures::Future;
use bifrost_hasher::hash_str;
use DISABLE_SHORTCUT;

lazy_static! {
    pub static ref DEFAULT_CLIENT_POOL: ClientPool = ClientPool::new();
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RPCRequestError {
    FunctionIdNotFound,
    ServiceIdNotFound,
    Other,
}

#[derive(Debug)]
pub enum RPCError {
    IOError(io::Error),
    RequestError(RPCRequestError),
}

pub trait RPCService: Sync + Send {
    fn dispatch(&self, data: Vec<u8>) -> Result<Vec<u8>, RPCRequestError>;
    fn register_shortcut_service(&self, service_ptr: usize, server_id: u64, service_id: u64);
}

pub struct Server {
    services: RwLock<HashMap<u64, Arc<RPCService>>>,
    pub address: String,
    pub server_id: u64
}

pub struct ClientPool {
    clients: Mutex<HashMap<String, Arc<RPCClient>>>
}

fn encode_res(res: Result<Vec<u8>, RPCRequestError>) -> Vec<u8> {
    match res {
        Ok(vec) => {
            [0u8; 1].iter().cloned().chain(vec.into_iter()).collect()
        },
        Err(e) => {
            let err_id = match e {
                RPCRequestError::FunctionIdNotFound => 1u8,
                RPCRequestError::ServiceIdNotFound => 2u8,
                _ => 255u8
            };
            vec!(err_id)
        }
    }
}

fn decode_res(res: io::Result<Vec<u8>>) -> Result<Vec<u8>, RPCError> {
    match res {
        Ok(res) => {
            if res[0] == 0u8 {
                Ok(res.into_iter().skip(1).collect())
            } else {
                match res[0] {
                    1u8 => Err(RPCError::RequestError(RPCRequestError::FunctionIdNotFound)),
                    2u8 => Err(RPCError::RequestError(RPCRequestError::ServiceIdNotFound)),
                    _ => Err(RPCError::RequestError(RPCRequestError::Other)),
                }
            }
        },
        Err(e) => Err(RPCError::IOError(e))
    }
}

impl Server {
    pub fn new(address: &String) -> Arc<Server> {
        Arc::new(Server {
            services: RwLock::new(HashMap::new()),
            address: address.clone(),
            server_id: hash_str(address)
        })
    }
    pub fn listen(server: &Arc<Server>) {
        let address = &server.address;
        let server = server.clone();
        tcp::server::Server::new(address, Box::new(move |data| {
            let (svr_id, data) = extract_u64_head(data);
            let svr_map = server.services.read();
            let service = svr_map.get(&svr_id);
            let res = match service {
                Some(service) => {
                    encode_res(service.dispatch(data))
                },
                None => encode_res(Err(RPCRequestError::ServiceIdNotFound) as Result<Vec<u8>, RPCRequestError>)
            };
            //println!("SVR RPC: {} - {}ms", svr_id, time::get_time() - t);
            res
        }));
    }
    pub fn listen_and_resume(server: &Arc<Server>) {
        let server = server.clone();
        thread::spawn(move|| {
            let server = server;
            Server::listen(&server);
        });
    }
    pub fn register_service<T>(&self, service_id: u64,  service: &Arc<T>)
    where T: RPCService + Sized + 'static{
        let service = service.clone();
        if !DISABLE_SHORTCUT {
            let service_ptr = Arc::into_raw(service.clone()) as usize;
            service.register_shortcut_service(service_ptr, self.server_id, service_id);
        } else {
            println!("SERVICE SHORTCUT DISABLED");
        }
        self.services.write().insert(service_id, service);
    }
    pub fn remove_service(&self, service_id: u64) {
        self.services.write().remove(&service_id);
    }
    pub fn address(&self) -> &String {
        &self.address
    }
}

pub struct RPCClient {
    client: Mutex<tcp::client::Client>,
    pub server_id: u64,
    pub address: String
}

impl RPCClient {
    pub fn send(&self, svr_id: u64, data: Vec<u8>) -> Result<Vec<u8>, RPCError> {
        decode_res(self.client.lock().send(prepend_u64(svr_id, data)))
    }
    pub fn send_async(&self, svr_id: u64, data: Vec<u8>) -> Box<Future<Item = Vec<u8>, Error = RPCError>> {
        Box::new(self.client.lock()
            .send_async(prepend_u64(svr_id, data))
            .then(move |res| decode_res(res)))
    }
    pub fn new(addr: &String) -> io::Result<Arc<RPCClient>> {
        let client = tcp::client::Client::connect(addr)?;
        Ok(Arc::new(RPCClient {
            server_id: client.server_id,
            client: Mutex::new(client),
            address: addr.clone()
        }))
    }
    pub fn with_timeout(addr: &String, timeout: Duration) -> io::Result<Arc<RPCClient>> {
        let client = tcp::client::Client::connect_with_timeout(addr, timeout)?;
        Ok(Arc::new(RPCClient {
            server_id: client.server_id,
            client: Mutex::new(client),
            address: addr.clone()
        }))
    }
}

impl ClientPool {
    pub fn new() -> ClientPool {
        ClientPool {
            clients: Mutex::new(HashMap::new())
        }
    }

    pub fn get(&self, addr: &String) -> io::Result<Arc<RPCClient>> {
        let mut clients = self.clients.lock();
        if clients.contains_key(addr) {
            Ok(clients.get(addr).unwrap().clone())
        } else {
            let client = RPCClient::new(addr);
            if let Ok(client) = client {
                clients.insert(addr.clone(), client.clone());
                Ok(client)
            } else {
                Err(client.err().unwrap())
            }
        }
    }
}