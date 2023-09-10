use lsp_types::notification::Notification;
use serde::{ser::SerializeMap, Deserialize, Serialize};

use crate::version::Version;

/// A JSON-RPC notification
pub struct NotificationMessage<N: Notification> {
    params: Option<N::Params>,
}

impl<N: Notification> NotificationMessage<N> {
    /// Constructs a JSON-RPC notification from its corresponding LSP type.
    pub fn new(params: Option<N::Params>) -> Self {
        NotificationMessage { params }
    }
}

impl<R: Notification> Serialize for NotificationMessage<R> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut message_map = serializer.serialize_map(Some(3))?;
        message_map.serialize_entry("jsonrpc", &Version)?;
        message_map.serialize_entry("method", R::METHOD)?;
        if self.params.is_some() {
            message_map.serialize_entry("params", &self.params)?;
        }
        message_map.end()
    }
}

impl<'de, R: Notification> Deserialize<'de> for NotificationMessage<R> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct NotificationMessageDom<R: Notification> {
            #[serde(rename = "jsonrpc")]
            _jsonrpc: Version,
            method: String,
            params: Option<R::Params>,
        }

        let request_messarge_dom = NotificationMessageDom::<R>::deserialize(deserializer)?;
        if request_messarge_dom.method != R::METHOD {
            return Err(serde::de::Error::unknown_variant(
                &request_messarge_dom.method,
                &[R::METHOD],
            ));
        }

        Ok(NotificationMessage {
            params: request_messarge_dom.params,
        })
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{notification::Cancel, CancelParams, NumberOrString};
    use once_cell::sync::Lazy;
    use serde_json::json;

    use super::*;

    impl<R: Notification> std::fmt::Debug for NotificationMessage<R> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NotificationMessage")
                .field("json", &Version)
                .field("method", &R::METHOD)
                .field("params", &serde_json::to_string(&self.params).unwrap())
                .finish()
        }
    }

    impl<R: Notification> PartialEq for NotificationMessage<R> {
        fn eq(&self, other: &Self) -> bool {
            const SERIALIZE_ERROR_MESSAGE: &str = "Params should be serializable into Value.";
            serde_json::to_value(&self.params).expect(SERIALIZE_ERROR_MESSAGE)
                == serde_json::to_value(&other.params).expect(SERIALIZE_ERROR_MESSAGE)
        }
    }

    const CANCEL_NOTIFICATION_MOCK: NotificationMessage<Cancel> = NotificationMessage {
        params: Some(CancelParams {
            id: NumberOrString::Number(0),
        }),
    };

    static CANCEL_NOTIFICATION_JSON: Lazy<serde_json::Value> = Lazy::new(|| {
        json!({
            "jsonrpc": "2.0",
            "method": "$/cancelRequest",
            "params": {
                "id": 0,
            }
        })
    });

    #[test]
    fn serializes_notification_message() {
        assert_eq!(
            *CANCEL_NOTIFICATION_JSON,
            serde_json::to_value(CANCEL_NOTIFICATION_MOCK).unwrap()
        )
    }

    #[test]
    fn deserializes_notification_message() {
        assert_eq!(
            CANCEL_NOTIFICATION_MOCK,
            serde_json::from_value::<NotificationMessage<Cancel>>(
                CANCEL_NOTIFICATION_JSON.clone(),
            )
            .unwrap()
        )
    }
}
