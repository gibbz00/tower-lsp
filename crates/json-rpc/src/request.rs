use lsp_types::request::Request;
use lsp_types::NumberOrString;
use serde::{ser::SerializeMap, Deserialize, Serialize};

use crate::version::Version;

pub struct RequestMessage<R: Request> {
    id: NumberOrString,
    params: Option<R::Params>,
}

impl<R: Request> RequestMessage<R> {
    pub fn new(id: NumberOrString, params: Option<R::Params>) -> Self {
        RequestMessage { id, params }
    }

    pub fn id(&self) -> &NumberOrString {
        &self.id
    }

    pub fn params(&self) -> Option<&R::Params> {
        self.params.as_ref()
    }

    pub fn into_parts(self) -> (NumberOrString, Option<R::Params>) {
        (self.id, self.params)
    }
}

impl<R: Request> Serialize for RequestMessage<R> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut message_map = serializer.serialize_map(Some(4))?;
        message_map.serialize_entry("jsonrpc", &Version)?;
        message_map.serialize_entry("id", &self.id)?;
        message_map.serialize_entry("method", R::METHOD)?;
        if self.params.is_some() {
            message_map.serialize_entry("params", &self.params)?;
        }
        message_map.end()
    }
}

impl<'de, R: Request> Deserialize<'de> for RequestMessage<R> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RequestMessageDom<R: Request> {
            #[serde(rename = "jsonrpc")]
            _jsonrpc: Version,
            id: NumberOrString,
            method: String,
            params: Option<R::Params>,
        }

        let request_messarge_dom = RequestMessageDom::<R>::deserialize(deserializer)?;
        if request_messarge_dom.method != R::METHOD {
            return Err(serde::de::Error::unknown_variant(
                &request_messarge_dom.method,
                &[R::METHOD],
            ));
        }

        Ok(RequestMessage {
            id: request_messarge_dom.id,
            params: request_messarge_dom.params,
        })
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{
        request::{Request, WillRenameFiles},
        RenameFilesParams,
    };
    use once_cell::sync::Lazy;
    use serde_json::json;

    use crate::{version::Version, RequestMessage};

    impl<R: Request> std::fmt::Debug for RequestMessage<R> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RequestMessage")
                .field("jsonrpc", &Version)
                .field("id", &self.id)
                .field("method", &R::METHOD)
                .field("params", &serde_json::to_string(&self.params).unwrap())
                .finish()
        }
    }

    impl<R: Request> PartialEq for RequestMessage<R> {
        fn eq(&self, other: &Self) -> bool {
            const SERIALIZE_ERROR_MESSAGE: &str = "Params should be serializable into Value.";
            serde_json::to_value(&self.params).expect(SERIALIZE_ERROR_MESSAGE)
                == serde_json::to_value(&other.params).expect(SERIALIZE_ERROR_MESSAGE)
                && self.id == other.id
        }
    }

    const WILL_RENAME_FILES_REQUEST_MOCK: RequestMessage<WillRenameFiles> = RequestMessage {
        id: lsp_types::NumberOrString::Number(0),
        params: Some(RenameFilesParams { files: vec![] }),
    };

    static WILL_RENAME_FILES_REQUEST_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
        json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "workspace/willRenameFiles",
            "params": {
                "files": []
            }
        })
    });

    #[test]
    fn serializes_request_message() {
        assert_eq!(
            *WILL_RENAME_FILES_REQUEST_JSON,
            serde_json::to_value(WILL_RENAME_FILES_REQUEST_MOCK).unwrap()
        )
    }

    #[test]
    fn deserializes_request_message() {
        assert_eq!(
            WILL_RENAME_FILES_REQUEST_MOCK,
            serde_json::from_value::<RequestMessage<WillRenameFiles>>(
                WILL_RENAME_FILES_REQUEST_JSON.clone(),
            )
            .unwrap()
        )
    }
}
