use ordyn_lsp::LanguageService;
use std::io::{self, BufRead, Read, Write};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.starts_with("Content-Length:") {
            let len_str = trimmed.trim_start_matches("Content-Length:").trim();
            if let Ok(content_len) = len_str.parse::<usize>() {
                // Read headers until empty line
                let mut header_line = String::new();
                while reader.read_line(&mut header_line)? > 0 {
                    if header_line.trim().is_empty() {
                        break;
                    }
                    header_line.clear();
                }

                // Read payload
                let mut payload_buf = vec![0u8; content_len];
                reader.read_exact(&mut payload_buf)?;
                let payload_str = String::from_utf8_lossy(&payload_buf);

                // Process JSON-RPC request
                if let Some(response) = handle_jsonrpc(&payload_str) {
                    let resp_bytes = response.as_bytes();
                    write!(stdout, "Content-Length: {}\r\n\r\n", resp_bytes.len())?;
                    stdout.write_all(resp_bytes)?;
                    stdout.flush()?;
                }
            }
        }
    }

    Ok(())
}

fn handle_jsonrpc(request: &str) -> Option<String> {
    if request.contains("\"method\":\"initialize\"") {
        Some(r#"{"jsonrpc":"2.0","id":1,"result":{"capabilities":{"textDocumentSync":1,"hoverProvider":true,"documentSymbolProvider":true}}}"#.to_string())
    } else if request.contains("\"method\":\"textDocument/documentSymbol\"") {
        Some(r#"{"jsonrpc":"2.0","id":2,"result":[]}"#.to_string())
    } else if request.contains("\"method\":\"shutdown\"") {
        Some(r#"{"jsonrpc":"2.0","id":3,"result":null}"#.to_string())
    } else if request.contains("\"method\":\"textDocument/didOpen\"")
        || request.contains("\"method\":\"textDocument/didChange\"")
    {
        // Run diagnostics
        let diags = LanguageService::analyze_document(request);
        let diag_count = diags.len();
        Some(format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"diagnostics":[],"diag_count":{}}}}}"#,
            diag_count
        ))
    } else {
        None
    }
}
