use lsp_types::{InitializeParams, InitializeResult, InitializedParams, MessageType};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tower_lsp::*;
use tower_lsp_json_rpc::Message;

struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    test_did_open_e2e().await;
}

async fn test_did_open_e2e() {
    let initialize = r#"{"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{"textDocumentSync":1}},"id":1}"#;

    let did_open = r#"{
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
              "textDocument": {
                "uri": "file:///foo.rs",
                "languageId": "rust",
                "version": 1,
                "text": "this is a\ntest fo typos\n"
              }
            }
          }
          "#;

    let (mut req_client, mut resp_client) = start_server();
    let mut buf = vec![0; 1024];

    req_client
        .write_all(req(initialize).as_bytes())
        .await
        .unwrap();
    let _ = resp_client.read(&mut buf).await.unwrap();

    tracing::info!("{}", did_open);
    req_client
        .write_all(req(did_open).as_bytes())
        .await
        .unwrap();
    let n = resp_client.read(&mut buf).await.unwrap();

    // assert_eq!(
    //     body(&buf[..n]).unwrap(),
    //     r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"diagnostics":[{"message":"`fo` should be `of`, `for`","range":{"end":{"character":7,"line":1},"start":{"character":5,"line":1}},"severity":2,"source":"typos-lsp"}],"uri":"file:///foo.rs","version":1}}"#,
    // )
}

fn start_server() -> (tokio::io::DuplexStream, tokio::io::DuplexStream) {
    let (rx, tx) = mpsc::unbounded_channel::<Message>();
    let (req_client, req_server) = tokio::io::duplex(1024);
    let (resp_server, resp_client) = tokio::io::duplex(1024);

    let (service, socket) = LspService::new(|client| Backend { client });

    // start server as concurrent task
    tokio::spawn(Server::new(req_server, resp_server, socket).serve(service));

    (req_client, resp_client)
}

fn req(msg: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg)
}
