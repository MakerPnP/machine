use std::time::Duration;

use ergot::net_stack::endpoints::EndpointClient;
use ergot::net_stack::{NetStackHandle, ReqRespError};
use ergot::traits::Endpoint;
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

pub struct ClientWrapper<'a, E: Endpoint, NS: NetStackHandle> {
    timeout: Duration,
    client: EndpointClient<'a, E, NS>,
}

impl<'a, E, NS> ClientWrapper<'a, E, NS>
where
    E: Endpoint,
    NS: NetStackHandle,
{
    pub fn new(timeout: Duration, client: EndpointClient<'a, E, NS>) -> Self {
        Self {
            timeout,
            client,
        }
    }

    pub async fn request(&self, req: &E::Request) -> Result<E::Response, ClientError>
    where
        E: Endpoint,
        E::Request: Serialize + Clone + DeserializeOwned + 'static,
        E::Response: Serialize + Clone + DeserializeOwned + 'static,
    {
        tokio::time::timeout(self.timeout, self.client.request(req))
            .await
            .map_err(|_e| ClientError::Timeout(self.timeout))
            .map(|r| r.map_err(|e| ClientError::RequestError(e)))?
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("timeout after {ms}ms", ms = .0.as_millis())]
    Timeout(Duration),
    #[error("Request error: {0:?}")]
    RequestError(ReqRespError),
}
