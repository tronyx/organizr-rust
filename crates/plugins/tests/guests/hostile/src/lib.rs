//! An extension that tries what an extension must not be able to do.

wit_bindgen::generate!({
    path: "../../../wit",
    world: "extension",
});

use organizr::extension::host;

struct Hostile;

fn text(body: String) -> Response {
    Response {
        status: 200,
        headers: Vec::new(),
        body: body.into_bytes(),
    }
}

impl Guest for Hostile {
    fn handle(request: Request) -> Response {
        match request.path.as_str() {
            // Work without an end.
            "/spin" => {
                let mut number: u64 = 1;
                loop {
                    number = number
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1);
                    if number == 0 {
                        break;
                    }
                }
                text("the loop ended".to_owned())
            }
            // More memory than a call can have.
            "/grow" => {
                let mut blocks: Vec<Vec<u8>> = Vec::new();
                for _ in 0..40 {
                    blocks.push(vec![1; 16 * 1024 * 1024]);
                }
                text(format!("{} blocks", blocks.len()))
            }
            "/panic" => panic!("the extension gives up"),
            // An answer that is larger than an answer can be.
            "/big" => Response {
                status: 200,
                headers: Vec::new(),
                body: vec![b'a'; 9 * 1024 * 1024],
            },
            "/files" => text(match std::fs::read_dir("/") {
                Ok(entries) => format!("folder with {} entries", entries.count()),
                Err(error) => format!("no folder: {error}"),
            }),
            "/write" => text(
                match std::fs::write("/tmp/from-the-extension.txt", "text") {
                    Ok(()) => "written".to_owned(),
                    Err(error) => format!("not written: {error}"),
                },
            ),
            "/network" => text(match std::net::TcpStream::connect("127.0.0.1:80") {
                Ok(_) => "connected".to_owned(),
                Err(error) => format!("no connection: {error}"),
            }),
            "/environment" => text(format!(
                "{} variables, {} arguments",
                std::env::vars().count(),
                std::env::args().count()
            )),
            "/flood" => {
                for line in 0..1000 {
                    host::log(host::LogLevel::Warn, &format!("line {line}"));
                }
                text("flooded".to_owned())
            }
            // As many values as the storage takes.
            "/fill" => {
                let value = "v".repeat(100_000);
                let mut stored = 0;
                let mut refusal = String::new();
                for number in 0..100 {
                    match host::set_value(&format!("key{number}"), &value) {
                        Ok(()) => stored += 1,
                        Err(error) => {
                            refusal = error;
                            break;
                        }
                    }
                }
                text(format!("{stored} stored; {refusal}"))
            }
            "/read" => text(format!(
                "{:?}",
                host::get_value("key0").map(|value| value.len())
            )),
            // A request to the address of the query, through the host.
            "/fetch" => text(
                match host::send_request(&host::HttpRequest {
                    method: "POST".to_owned(),
                    url: request.query.clone(),
                    headers: vec![("x-from".to_owned(), "hostile".to_owned())],
                    body: b"ping".to_vec(),
                }) {
                    Ok(answer) => format!(
                        "{} {}",
                        answer.status,
                        String::from_utf8_lossy(&answer.body)
                    ),
                    Err(error) => format!("refused: {error}"),
                },
            ),
            // More requests than a call may send.
            "/fetch-many" => {
                let mut sent = 0;
                let mut refusal = String::new();
                for _ in 0..30 {
                    let one = host::HttpRequest {
                        method: "GET".to_owned(),
                        url: request.query.clone(),
                        headers: Vec::new(),
                        body: Vec::new(),
                    };
                    match host::send_request(&one) {
                        Ok(_) => sent += 1,
                        Err(error) => {
                            refusal = error;
                            break;
                        }
                    }
                }
                text(format!("{sent} sent; {refusal}"))
            }
            // A body that is larger than a request can have.
            "/fetch-big" => text(
                match host::send_request(&host::HttpRequest {
                    method: "POST".to_owned(),
                    url: request.query.clone(),
                    headers: Vec::new(),
                    body: vec![b'a'; 2 * 1024 * 1024],
                }) {
                    Ok(answer) => format!("{}", answer.status),
                    Err(error) => format!("refused: {error}"),
                },
            ),
            "/forget" => {
                host::delete_value("key0");
                text("deleted".to_owned())
            }
            _ => Response {
                status: 404,
                headers: Vec::new(),
                body: Vec::new(),
            },
        }
    }
}

export!(Hostile);
