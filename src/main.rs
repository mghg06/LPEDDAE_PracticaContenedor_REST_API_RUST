/*
MGHG
Laboratorio para el despliegue de aplicaciones empresariales 
Practica de API RUST en contenedor DOCKER 
*/

//Import depedences
use postgres::{ Client, NoTls };
use postgres::Error as PostgresError;
use std::net::{ TcpListener, TcpStream };
use std::io::{ Read, Write };
use std::env;

#[macro_use]
extern crate serde_derive;

//Model: Helado struct with id, sabor, stock
#[derive(Serialize, Deserialize)]
struct Helado {
    id: Option<i32>,
    sabor: String,
    stock: String,
}

//DATABASE URL
const DB_URL: &str = env!("DATABASE_URL");

//constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_ERROR: &str = "HTTP/1.1 500 INTERNAL ERROR\r\n\r\n";

//main function
fn main() {
    //Set Database
    if let Err(_) = set_database() {
        println!("Error setting database");
        return;
    }

    //start server and print port
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server listening on port 8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Unable to connect: {}", e);
            }
        }
    }
}

//handle requests
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /helados") => handle_post_request(r),
                r if r.starts_with("GET /helados/") => handle_get_request(r),
                r if r.starts_with("GET /helados") => handle_get_all_request(r),
                r if r.starts_with("PUT /helados/") => handle_put_request(r),
                r if r.starts_with("DELETE /helados/") => handle_delete_request(r),
                _ => (NOT_FOUND.to_string(), "404 not found".to_string()),
            };

            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => eprintln!("Unable to read stream: {}", e),
    }
}

//handle post request
fn handle_post_request(request: &str) -> (String, String) {
    match (get_helado_request_body(&request), Client::connect(DB_URL, NoTls)) {
        (Ok(helado), Ok(mut client)) => {
            client
                .execute(
                    "INSERT INTO helados (sabor, stock) VALUES ($1, $2)",
                    &[&helado.sabor, &helado.stock]
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "Helado created".to_string())
        }
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

//handle get request
fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) =>
            match client.query_one("SELECT * FROM helados WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let helado = Helado {
                        id: row.get(0),
                        sabor: row.get(1),
                        stock: row.get(2),
                    };

                    (OK_RESPONSE.to_string(), serde_json::to_string(&helado).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "Helado not found".to_string()),
            }

        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

//handle get all request
fn handle_get_all_request(_request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut helados = Vec::new();

            for row in client.query("SELECT id, sabor, stock FROM helados", &[]).unwrap() {
                helados.push(Helado {
                    id: row.get(0),
                    sabor: row.get(1),
                    stock: row.get(2),
                });
            }

            (OK_RESPONSE.to_string(), serde_json::to_string(&helados).unwrap())
        }
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

//handle put request
fn handle_put_request(request: &str) -> (String, String) {
    match
        (
            get_id(&request).parse::<i32>(),
            get_helado_request_body(&request),
            Client::connect(DB_URL, NoTls),
        )
    {
        (Ok(id), Ok(helado), Ok(mut client)) => {
            client
                .execute(
                    "UPDATE helados SET sabor = $1, stock = $2 WHERE id = $3",
                    &[&helado.sabor, &helado.stock, &id]
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "Helado updated".to_string())
        }
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

//handle delete request
fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client.execute("DELETE FROM helados WHERE id = $1", &[&id]).unwrap();

            //if rows affected is 0, helado not found
            if rows_affected == 0 {
                return (NOT_FOUND.to_string(), "Helado not found".to_string());
            }

            (OK_RESPONSE.to_string(), "Helado deleted".to_string())
        }
        _ => (INTERNAL_ERROR.to_string(), "Internal error".to_string()),
    }
}

//db setup
fn set_database() -> Result<(), PostgresError> {
    let mut client = Client::connect(DB_URL, NoTls)?;
    client.batch_execute(
        "
        CREATE TABLE IF NOT EXISTS helados (
            id SERIAL PRIMARY KEY,
            sabor VARCHAR NOT NULL,
            stock VARCHAR NOT NULL
        )
    "
    )?;
    Ok(())
}

//Get id from request URL
fn get_id(request: &str) -> &str {
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

//deserialize helado from request body without id
fn get_helado_request_body(request: &str) -> Result<Helado, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}