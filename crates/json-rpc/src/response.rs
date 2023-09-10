use lsp_types::request::Request;
use serde::{ser::SerializeMap, Deserialize, Serialize};

use crate::{version::Version, Error, Result};

#[derive(Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseId {
    Number(i32),
    String(String),
    /// While `null` is considered a valid request ID by the JSON-RPC 2.0 specification, its use is
    /// _strongly_ discouraged because the specification also uses a `null` value to indicate an
    /// unknown ID in the [`Response`] object.
    Null,
}

pub struct ResponseMessage<R: Request> {
    id: ResponseId,
    kind: Result<R::Result>,
}

impl<R: Request> ResponseMessage<R> {
    pub fn new(id: ResponseId, kind: Result<R::Result>) -> Self {
        ResponseMessage { id, kind }
    }

    pub fn into_parts(self) -> (ResponseId, Result<R::Result>) {
        (self.id, self.kind)
    }

    pub fn id(&self) -> &ResponseId {
        &self.id
    }

    pub fn kind(&self) -> std::result::Result<&R::Result, &Error> {
        self.kind.as_ref()
    }
}

impl<R: Request> Serialize for ResponseMessage<R> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut message_map = serializer.serialize_map(Some(3))?;
        message_map.serialize_entry("jsonrpc", &Version)?;
        message_map.serialize_entry("id", &self.id)?;
        match &self.kind {
            Ok(value) => message_map.serialize_entry("result", value)?,
            Err(err) => message_map.serialize_entry("error", err)?,
        }
        message_map.end()
    }
}

impl<'de, R: Request> Deserialize<'de> for ResponseMessage<R> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ResponseMessageDom<R: Request> {
            #[serde(rename = "jsonrpc")]
            _jsonrpc: Version,
            id: ResponseId,
            #[serde(flatten, bound = "R: Request")]
            kind: ResultDom<R>,
        }

        #[derive(Deserialize)]
        enum ResultDom<R: Request> {
            #[serde(rename = "result")]
            Ok(R::Result),
            #[serde(rename = "error")]
            Error(Error),
        }

        let response_messarge_dom = ResponseMessageDom::<R>::deserialize(deserializer)?;

        Ok(ResponseMessage {
            id: response_messarge_dom.id,
            kind: match response_messarge_dom.kind {
                ResultDom::Ok(value) => Ok(value),
                ResultDom::Error(err) => Err(err),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::request::Shutdown;
    use once_cell::sync::Lazy;
    use serde_json::json;

    use super::*;

    impl<R: Request> std::fmt::Debug for ResponseMessage<R> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut debug_struct = f.debug_struct("ResponseMessage");
            debug_struct
                .field("jsonrpc", &Version)
                .field("id", &self.id);
            match &self.kind {
                Ok(value) => debug_struct.field("result", &serde_json::to_string(value).unwrap()),
                Err(err) => debug_struct.field("error", err),
            };

            debug_struct.finish()
        }
    }

    impl<R: Request> PartialEq for ResponseMessage<R> {
        fn eq(&self, other: &Self) -> bool {
            const SERIALIZE_ERROR_MESSAGE: &str = "Kind should be serializable into Value.";
            serde_json::to_value(&self.kind).expect(SERIALIZE_ERROR_MESSAGE)
                == serde_json::to_value(&other.kind).expect(SERIALIZE_ERROR_MESSAGE)
                && self.id == other.id
        }
    }

    const SHUTDOWN_RESPONSE_MOCK: ResponseMessage<Shutdown> = ResponseMessage {
        id: ResponseId::Number(0),
        kind: Ok(()),
    };

    static SHUTDOWN_RESPONSE_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
        json!({
            "jsonrpc": "2.0",
            "id": 0,
            "result": null
        })
    });

    #[test]
    fn serializes_request_message() {
        assert_eq!(
            *SHUTDOWN_RESPONSE_JSON,
            serde_json::to_value(SHUTDOWN_RESPONSE_MOCK).unwrap()
        )
    }

    #[test]
    fn deserializes_request_message() {
        assert_eq!(
            SHUTDOWN_RESPONSE_MOCK,
            serde_json::from_value::<ResponseMessage<Shutdown>>(SHUTDOWN_RESPONSE_JSON.clone(),)
                .unwrap()
        )
    }
}
