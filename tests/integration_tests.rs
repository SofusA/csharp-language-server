use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::{env, fs, thread};
use url::Url;

static SEQUENTIAL: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

#[test]
fn first_line_is_jsonrpc() {
    let _shared = SEQUENTIAL.lock().unwrap();
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    let stdout = cmd.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let first_line = lines
        .next()
        .expect("No output received")
        .expect("Failed to read line");

    cmd.kill().unwrap();

    // language server responds with a jsonrpc message
    assert!(first_line.contains("Content-Length"));
    cmd.wait().unwrap();
}

fn write_lsp_message<W: Write>(mut w: W, body: &[u8]) -> io::Result<()> {
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(body)?;
    w.flush()?;
    Ok(())
}

fn read_next_lsp_json<R: Read>(reader: &mut BufReader<R>) -> io::Result<Option<Value>> {
    // parse headers
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None); // EOF
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length:")
            && let Ok(length) = rest.trim().parse::<usize>()
        {
            content_length = Some(length);
        }
    }

    // read body
    let len = match content_length {
        Some(n) => n,
        None => return Ok(None),
    };
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;

    // parse body
    match serde_json::from_slice::<Value>(&body) {
        Ok(json) => Ok(Some(json)),
        Err(e) => {
            eprintln!(
                "JSON parse error: {e}\nBody: {}",
                String::from_utf8_lossy(&body)
            );
            Ok(None)
        }
    }
}

#[test]
fn provides_diagnostics() {
    let _shared = SEQUENTIAL.lock().unwrap();

    let mut child = Command::new(assert_cmd::cargo::cargo_bin!())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut child_stdin = child.stdin.take().expect("no stdin");
    let child_stdout = child.stdout.take().expect("no stdout");
    let child_stderr = child.stderr.take().expect("no stderr");

    let _stderr_thread = thread::spawn(move || {
        let mut r = BufReader::new(child_stderr);
        let mut buf = [0u8; 8192];
        loop {
            match r.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = std::io::stderr().write_all(&buf[..n]);
                    let _ = std::io::stderr().flush();
                }
                Err(e) => {
                    eprintln!("stderr read error: {e}");
                    break;
                }
            }
        }
    });

    let test_project_directory = {
        let mut dir = env::current_dir().unwrap();
        dir.push("tests");
        dir.push("TestProject");

        dir
    };

    let document_path = {
        let mut path = test_project_directory.clone();
        path.push("Program.cs");
        path
    };

    let mut out = BufReader::new(child_stdout);

    let doc_text = fs::read_to_string(&document_path).unwrap();
    let doc_uri = Url::from_file_path(&document_path).unwrap().to_string();
    let workspace_uri = Url::from_file_path(&test_project_directory)
        .unwrap()
        .to_string();

    let workspace_name = "TestProject";
    let initialize = initialize_message(&workspace_uri, workspace_name);

    write_lsp_message(&mut child_stdin, &serde_json::to_vec(&initialize).unwrap()).unwrap();
    println!("[send] initialize(id=0)");

    let mut diagnostic_response = Value::Null;

    loop {
        match read_next_lsp_json(&mut out).unwrap() {
            None => {
                eprintln!("child stdout EOF");
                break;
            }
            Some(msg) => {
                println!("[recv] {}", serde_json::to_string_pretty(&msg).unwrap());

                if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                    // client/registerCapability -> respond {result:null}
                    if method == "client/registerCapability" {
                        if let Some(req_id) = msg.get("id") {
                            let resp =
                                json!({"jsonrpc":"2.0","result":Value::Null,"id": req_id.clone()});
                            write_lsp_message(
                                &mut child_stdin,
                                &serde_json::to_vec(&resp).unwrap(),
                            )
                            .unwrap();
                            println!("[send] registerCapability response id={}", req_id);
                        }
                        continue;
                    }

                    // workspace/configuration -> respond array matching items.len()
                    if method == "workspace/configuration" {
                        let items = msg
                            .get("params")
                            .and_then(|p| p.get("items"))
                            .and_then(|i| i.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                        if let Some(req_id) = msg.get("id") {
                            // Return `null` for each item (or `{}` if you prefer)
                            let result: Vec<Value> = (0..items).map(|_| Value::Null).collect();
                            let resp =
                                json!({"jsonrpc":"2.0","result": result, "id": req_id.clone()});
                            write_lsp_message(
                                &mut child_stdin,
                                &serde_json::to_vec(&resp).unwrap(),
                            )
                            .unwrap();
                            println!(
                                "[send] workspace/configuration response with {items} item(s)"
                            );
                        }
                        continue;
                    }

                    if method == "workspace/workspaceFolders" {
                        if let Some(req_id) = msg.get("id") {
                            let resp = json!({
                                "jsonrpc":"2.0",
                                "result":[{"name": workspace_name, "uri": workspace_uri}],
                                "id": req_id.clone()
                            });
                            write_lsp_message(
                                &mut child_stdin,
                                &serde_json::to_vec(&resp).unwrap(),
                            )
                            .unwrap();
                            println!("[send] workspaceFolders response");
                        }
                        continue;
                    }

                    if method == "window/workDoneProgress/create" {
                        if let Some(req_id) = msg.get("id") {
                            let resp =
                                json!({"jsonrpc":"2.0","result":Value::Null,"id": req_id.clone()});
                            write_lsp_message(
                                &mut child_stdin,
                                &serde_json::to_vec(&resp).unwrap(),
                            )
                            .unwrap();
                            println!("[send] workDoneProgress/create response");
                        }
                        continue;
                    }

                    // workspace/projectInitializationComplete -> send didOpen and diagnostics
                    if method == "workspace/projectInitializationComplete" {
                        println!("[event] projectInitializationComplete");

                        let did_open = json!({
                            "jsonrpc":"2.0",
                            "method":"textDocument/didOpen",
                            "params":{
                              "textDocument":{
                                "languageId":"csharp",
                                "text": doc_text,
                                "uri": doc_uri,
                                "version": 0
                              }
                            }
                        });
                        write_lsp_message(
                            &mut child_stdin,
                            &serde_json::to_vec(&did_open).unwrap(),
                        )
                        .unwrap();
                        println!("[send] didOpen({doc_uri})");

                        let diagnostic_request = json!({
                          "jsonrpc":"2.0",
                          "method":"textDocument/diagnostic",
                          "params": {
                            "previousResultId": Value::Null,
                            "textDocument": { "uri": doc_uri }
                          },
                          "id": 1
                        });
                        write_lsp_message(
                            &mut child_stdin,
                            &serde_json::to_vec(&diagnostic_request).unwrap(),
                        )
                        .unwrap();
                        println!("[send] textDocument/diagnostic(id=1)");
                        continue;
                    }
                }

                // initialize response (id=0) -> send initialized
                if msg.get("id").and_then(|id| id.as_i64()) == Some(0) {
                    let initialized = json!({"jsonrpc":"2.0","method":"initialized","params":{}});
                    write_lsp_message(&mut child_stdin, &serde_json::to_vec(&initialized).unwrap())
                        .unwrap();
                    println!("[send] initialized");

                    continue;
                }

                // --- Diagnostics response (id=1) ---
                if msg.get("id").and_then(|id| id.as_i64()) == Some(1) {
                    let result = &msg["result"];
                    diagnostic_response = result.clone();
                    break;
                }
            }
        }
    }

    let diagnostic_response = diagnostic_response
        .get("items")
        .unwrap()
        .as_array()
        .unwrap();

    println!(
        "Diagnostics result:\n{}",
        serde_json::to_string_pretty(&diagnostic_response).unwrap()
    );

    assert_eq!(diagnostic_response.len(), 3);

    let missing_colon = get_diagnostic(diagnostic_response, "CS1002").unwrap();
    assert_eq!(missing_colon.get("message").unwrap(), "; expected");

    let unnecessary_using = get_diagnostic(diagnostic_response, "IDE0005").unwrap();
    assert_eq!(
        unnecessary_using.get("message").unwrap(),
        "Using directive is unnecessary."
    );

    let unused = get_diagnostic(diagnostic_response, "CS0029").unwrap();
    assert_eq!(
        unused.get("message").unwrap(),
        "Cannot implicitly convert type 'string' to 'int'"
    );

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}

fn get_diagnostic<'a>(response: &'a [Value], code: &str) -> Option<&'a Value> {
    response
        .iter()
        .find(|diag| diag.get("code").unwrap() == code)
}

fn initialize_message(workspace_uri: &str, workspace_name: &str) -> Value {
    json!({
      "jsonrpc":"2.0",
      "method":"initialize",
      "params":{
        "capabilities":{
          "general":{"positionEncodings":["utf-8","utf-32","utf-16"]},
          "textDocument":{
            "codeAction":{
              "codeActionLiteralSupport":{
                "codeActionKind":{"valueSet":["","quickfix","refactor","refactor.extract","refactor.inline","refactor.rewrite","source","source.organizeImports"]}
              },
              "dataSupport":true,
              "disabledSupport":true,
              "isPreferredSupport":true,
              "resolveSupport":{"properties":["edit","command"]}
            },
            "completion":{
              "completionItem":{
                "deprecatedSupport":true,
                "insertReplaceSupport":true,
                "resolveSupport":{"properties":["documentation","detail","additionalTextEdits"]},
                "snippetSupport":true,
                "tagSupport":{"valueSet":[1]}
              },
              "completionItemKind":{}
            },
            "diagnostic":{"dynamicRegistration":false,"relatedDocumentSupport":true},
            "formatting":{"dynamicRegistration":false},
            "hover":{"contentFormat":["markdown"]},
            "inlayHint":{"dynamicRegistration":false},
            "publishDiagnostics":{"tagSupport":{"valueSet":[1,2]},"versionSupport":true},
            "rename":{"dynamicRegistration":false,"honorsChangeAnnotations":false,"prepareSupport":true},
            "signatureHelp":{"signatureInformation":{"activeParameterSupport":true,"documentationFormat":["markdown"],"parameterInformation":{"labelOffsetSupport":true}}}
          },
          "window":{"workDoneProgress":true},
          "workspace":{
            "applyEdit":true,
            "configuration":true,
            "diagnostic":{"refreshSupport":true},
            "didChangeConfiguration":{"dynamicRegistration":false},
            "didChangeWatchedFiles":{"dynamicRegistration":true,"relativePatternSupport":false},
            "executeCommand":{"dynamicRegistration":false},
            "fileOperations":{"didRename":true,"willRename":true},
            "inlayHint":{"refreshSupport":false},
            "symbol":{"dynamicRegistration":false},
            "workspaceEdit":{
              "documentChanges":true,
              "failureHandling":"abort",
              "normalizesLineEndings":false,
              "resourceOperations":["create","rename","delete"]
            },
            "workspaceFolders":true
          }
        },
        "clientInfo":{"name":"helix","version":"25.07.1 (de0518d0)"},
        "processId": std::process::id(),
        "rootPath": "/var/home/sofusa/git/TestProj2",
        "rootUri": workspace_uri,
        "workspaceFolders":[{"name": workspace_name, "uri": workspace_uri}]
      },
      "id": 0
    })
}
