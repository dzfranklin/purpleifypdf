use anyhow::anyhow;
use purpleifypdf::{
    pdf_to_pdf::{transform, Update},
    Color, Quality,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::{self, Read, Write};

fn main() {
    handle_pdf().unwrap_or_else(|err| {
        // Ignore any error sending the error report to avoid infinite loop
        send(
            b"ERRR",
            &ErrorMessage {
                message: format!("{:?}", err),
            },
        )
        .ok();
    })
}

fn handle_pdf() -> Result<(), anyhow::Error> {
    let mut options: Option<Options> = None;

    let mut header_buf = [0; 4];
    let mut stdin = io::stdin();
    loop {
        match stdin.read_exact(&mut header_buf) {
            Ok(_) => {
                let body = receive(header_buf, &mut stdin)?;
                let (category, body) = parse_category(&body);
                match category {
                    b"OPTS" => options = Some(serde_json::from_slice(body)?),
                    b"DONE" => {
                        let options = options.ok_or_else(|| anyhow!("Missing options"))?;

                        let in_blob = fs::read(&options.in_file)?;

                        let mut state = transform(
                            in_blob,
                            None,
                            options.quality,
                            Some(options.background_color),
                        )?;

                        loop {
                            match state.next() {
                                Update::Progress(progress) => {
                                    send(
                                        b"STAT",
                                        &Status {
                                            percent_done: progress.percent_done(),
                                        },
                                    )?;
                                    state = progress;
                                }
                                Update::Complete(result) => {
                                    let complete = result?;
                                    let original_title = complete.original_title().to_string();

                                    fs::write(&options.out_file, complete.into_bytes())?;

                                    send(b"DONE", &Complete { original_title })?;
                                    break;
                                }
                            }
                        }

                        return Ok(());
                    }
                    _ => {
                        return Err(anyhow!(format!(
                            "Unrecognized message category: {:?}",
                            category
                        )))
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                // Erlang requires we die cleanly if stdin is closed
                return Ok(());
            }
            Err(err) => return Err(anyhow!(format!("Error reading from stdin: {:?}", err))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Options {
    quality: Quality,
    background_color: Color,
    in_file: String,
    out_file: String,
}

#[derive(Debug, Serialize)]
struct Status {
    percent_done: f64,
}

#[derive(Debug, Serialize)]
struct Complete {
    original_title: String,
}

#[derive(Debug, Serialize)]
struct ErrorMessage {
    message: String,
}

fn parse_category(body: &[u8]) -> (&[u8], &[u8]) {
    let category = &body[0..4];
    let body = &body[4..];
    (category, body)
}

fn send<T>(category: &[u8], data: &T) -> Result<(), io::Error>
where
    T: ?Sized + Serialize,
{
    let data = &serde_json::to_vec(data)?;
    let mut body = Vec::with_capacity(category.len() + data.len());
    body.extend_from_slice(category);
    body.extend_from_slice(data);

    let header = (body.len() as u32).to_be_bytes();

    io::stdout().write_all(&header)?;
    io::stdout().write_all(&body)?;

    io::stdout().flush()?;

    Ok(())
}

fn receive(header_buf: [u8; 4], stdin: &mut impl Read) -> Result<Vec<u8>, io::Error> {
    let mut body = vec![0; u32::from_be_bytes(header_buf) as usize];
    stdin.read_exact(&mut body)?;
    Ok(body)
}
