use std::io::Cursor;

use archive::Archive;
use cmp::{cdn::{CdnId, string_to_content_type, self, CdnData}, errors::Error};
use database::init_database;
use rocket::{http::{Status, ContentType}, Response, Request, response};
use sqlx::{Pool, MySql};
use rocket::response::Responder;

#[macro_use]
extern crate rocket;

mod archive;
mod database;
mod cmp;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/cdn/<dir>")]
async fn get_cdn_test<'r>(pool: &rocket::State<Pool<MySql>>, dir: String) -> Result<CdnData, Error> {
    match cdn::route(&pool, &dir).await {
        Ok(d) => Ok(d),
        Err(1) | Err(2) => Err(
            Error::new(Status::NotAcceptable, "Cannot parse hash from the route".to_string(), "Try the form \"/cdn/<hash>.<extension>\"".to_string()),
        ),
        Err(3) => Err(
            Error::new(Status::InternalServerError, "Unable to acquire intern connection".to_string(), "Retry later".to_string()),
        ),
        _ => Err(
            Error::new(Status::NotFound, "No ressource found at this address".to_string(), "Check that your hash is the good one".to_string()),
        ),
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // get archive & database connection
    let archive = Archive::from_file(&"archive.sfa".to_string(), "0.0.1".to_string(), true, true).expect("Cannot load archive"); // sfa = secured file archive

    let pool = init_database(&archive).await;

    {
        let mut conn = pool.acquire().await.unwrap();
        for file in std::fs::read_dir("/home/sed/Code/Helix/cdn").unwrap() {
            let buf = std::fs::read(file.as_ref().unwrap().path()).unwrap();
            let extension = &file.as_ref().unwrap().path();
            let extension = extension.extension().unwrap();
            let cdn_data = crate::cmp::cdn::CdnData::new(buf.as_slice(), string_to_content_type(format!("{}", extension.to_str().unwrap())));
        
            let req = cdn_data.save_to_sql(&mut conn).await;
            if req.is_err() {}
            else { println!("file {:?} added to cdn with hash {:?}", &file.unwrap().file_name(), &cdn_data.hash) }
        }
    
        //let hash = "0db653a2ba2ad65d77fb55edb9ceb5cc700ade28e0e8576768529a5b25548136";
        //let cdn = crate::cmp::cdn::CdnData::from_hash(CdnId::new(hash.to_string()), &mut conn).await.unwrap();
        //
        //std::fs::write(format!("./test.{}", cdn.extension), cdn.buf).unwrap();
    }

    // launch api
    let _rocket = rocket::build()
        .manage(pool)
        .mount("/", routes![index, get_cdn_test])
        .launch()
        .await?;

    Ok(())
}