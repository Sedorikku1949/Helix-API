use std::io::{Read, Cursor};
use rocket::{Request, response, Response, http::ContentType};
use rocket::response::Responder;
use sha3::{Sha3_256, Digest};
use sqlx::{MySqlConnection, Row, Pool, MySql};

fn gen_hash(buf: &[u8]) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(buf);
    let hash = hasher.finalize();

    format!("{:x}", hash)
}

#[derive(Debug)]
pub struct CdnId(String);

impl CdnId {
    pub fn new(hash: String) -> Self {
        assert!(hash.len() != 32, "hash ID must be a 32 caracters long string");
        Self(hash)
    }
}

#[derive(Debug)]
pub struct CdnData {
    pub hash: CdnId,
    pub buf: Box<[u8]>,
    pub extension: ContentType,
}


impl CdnData {
    pub async fn save_to_sql(&'_ self, conn: &mut MySqlConnection) -> Result<(), sqlx::error::Error> {
        let _ = sqlx::query("INSERT INTO `cdn` (hash, bin, extension) VALUES (?, ?, ?);")
            .bind(self.hash.0.clone())
            .bind(self.buf.as_ref())
            .bind(content_type_to_string(self.extension.clone()))
            .execute(conn)
            .await?;

        Ok(())
    }

    pub fn new(buffer: &[u8], extension: ContentType) -> CdnData {
        CdnData {
            hash: CdnId::new(gen_hash(buffer)),
            buf: buffer.into(),
            extension
        }
    }

    pub async fn from_hash(hash: CdnId, conn: &mut MySqlConnection) -> Result<CdnData, sqlx::error::Error> {
        let q = sqlx::query("SELECT * FROM cdn WHERE `hash`=?")
            .bind(hash.0)
            .fetch_one(conn)
            .await?;

        let extension = (&q.try_get::<String, _>("extension")?).clone();
        let bin = (&q.try_get::<Vec<u8>, _>("bin")?).clone();
        let hash = q.try_get::<String, _>("hash")?;

        drop(q);

        Ok(Self {
            hash: CdnId(hash),
            buf: bin.into_boxed_slice(),
            extension: string_to_content_type(extension) 
        })
    }

    pub async fn from_hash_and_extension(hash: CdnId, extension: String, conn: &mut MySqlConnection) -> Result<CdnData, sqlx::error::Error> {

        let q = sqlx::query("SELECT * FROM cdn WHERE `hash`=? AND `extension`=?;")
            .bind(hash.0)
            .bind(extension)
            .fetch_one(conn)
            .await?;

        let extension = (&q.try_get::<String, _>("extension")?).clone();
        let bin = (&q.try_get::<Vec<u8>, _>("bin")?).clone();
        let hash = q.try_get::<String, _>("hash")?;

        drop(q);
        
        Ok(Self {
            hash: CdnId(hash),
            buf: bin.into_boxed_slice(),
            extension: string_to_content_type(extension)
        })
    }
}

impl Read for CdnData {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
}

pub fn string_to_content_type(source: String) -> ContentType {
    match source.as_str() {
        "jpg" | "jpeg" => ContentType::JPEG,
        "png" => ContentType::PNG,
        _ => ContentType::Plain
    }
}

pub fn content_type_to_string(source: ContentType) -> String {
    if let Some(ext) = source.extension() { ext.to_string().to_lowercase() }
    else { "plain".to_string() }
    
}

impl<'r> Responder<'r, 'r> for CdnData {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        Response::build()
            .streamed_body(Cursor::new(self.buf))
            .header(self.extension)
            .ok()
    }
}

pub async fn route(pool: &Pool<MySql>, dir: &String) -> Result<CdnData, u8> {
    let mut query = dir.splitn(2, ".");
    let hash = if let Some(h) = query.next() { h } else { return Err(1) };
    let extension = if let Some(ext) = query.next() { ext.to_string() } else { return Err(2) };

    // acquire connection
    let conn = pool.acquire().await;
    if conn.is_err() { return Err(3) }
    let mut conn = conn.unwrap();

    // get cdn and return stream
    let cdn = crate::cmp::cdn::CdnData::from_hash_and_extension(CdnId::new(hash.to_string()), extension, &mut conn).await;

    if let Ok(data) = cdn { Ok(data) }
    else { Err(4) }
}