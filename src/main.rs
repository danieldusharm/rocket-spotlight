#[macro_use]
extern crate rocket;
mod paste_id;

use rocket::data::{Data, ToByteUnit};
use rocket::State;

use redis::Commands;
use std::env;

use paste_id::PasteId;

fn build_redis_client() -> redis::Client {
    let redis_host_name = match env::var("REDIS_HOSTNAME") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("Failed to retrieve REDIS_HOSTNAME");
            "localhost".to_string()
        }
    };

    let redis_password = match env::var("REDIS_PASSWORD") {
        Ok(val) => val,
        Err(_) => {
            eprintln!("No password set");
            "".to_string()
        }
    };

    let redis_conn_url = format!("{}://:{}@{}", "redis", redis_password, redis_host_name);

    redis::Client::open(redis_conn_url).expect("Invalid connection URL")
}

#[get("/<id>")]
fn retrieve(client: &State<redis::Client>, id: PasteId<'_>) -> String {
    let mut conn = client
        .get_connection()
        .expect("Failed to get Redis connection");

    let value = id.0.clone().into_owned();

    match conn.exists(&value) {
        Ok(true) => conn.get(&value).expect("Error occurred when getting value"),
        Ok(false) => "No value stored here".to_owned(),
        Err(_) => "Error checking if value exists".to_owned(),
    }
}

#[post("/", data = "<paste>")]
async fn store(client: &State<redis::Client>, paste: Data<'_>) -> Result<String, String> {
    let mut conn = match client.get_connection() {
        Ok(conn) => conn,
        Err(_) => {
            return Err("Failed to get Redis connection".to_string());
        }
    };

    // generate a new ID and clone the data
    let id = PasteId::new(5).0.clone().into_owned();

    // SET value from a 128 KiB datastream buffer
    match paste.open(128.kibibytes()).into_string().await {
        Ok(val) => {
            match redis::cmd("SET")
                .arg(&id)
                .arg(&val.into_inner())
                .query::<()>(&mut conn)
            {
                _ => (),
            };

            Ok(format!(
                "https://8000-danieldusha-rocketspotl-a1aloqri72e.ws-us107.gitpod.io/{}",
                &id
            ))
        }
        Err(_) => Err("Problems storing param data".to_string()),
    }
}

#[get("/")]
fn index() -> &'static str {
    "
    USAGE

      POST /

          accepts raw data in the body of the request and responds with a URL of
          a page containing the body's content

      GET /<id>

          retrieves the content for the paste with id `<id>`
    "
}

#[launch]
fn rocket() -> _ {
    let redis_client = build_redis_client();
    rocket::build()
        .manage(redis_client)
        .mount("/", routes![index, retrieve, store])
}
