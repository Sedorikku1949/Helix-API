use std::process::exit;

use sqlx::{Pool, MySql, mysql::MySqlPoolOptions};

use crate::archive::{Archive, self};

#[derive(Debug)]
pub enum SqlDatabaseError {
	InvalidConnectionDetails
}

fn format_conn_url(archive: &Archive) -> Result<String, SqlDatabaseError> {
    let user = archive.get("ConnectionHandler", "sql_user");
    let password = archive.get("ConnectionHandler", "sql_password");
    let host = archive.get("ConnectionHandler", "sql_host");
    let database = archive.get("ConnectionHandler", "sql_database");

    if user.is_string() && password.is_string() && host.is_string() &&  database.is_string() {
        Ok(
            format!(
                "mysql://{u}:{p}@{h}/{d}",
                u = user.as_str().unwrap_or(""),
                p = archive::medium_encryption::decrypt(&password.as_str().unwrap_or("")),
                h = host.as_str().unwrap_or(""),
                d = database.as_str().unwrap_or(""),
            )
        )
    } else {
        Err(SqlDatabaseError::InvalidConnectionDetails)
    }
}

pub async fn init_database(archive: &Archive) -> Pool<MySql> {
    let try_conn = format_conn_url(archive);
    match try_conn {
        Ok(conn) => {
            let try_pool = MySqlPoolOptions::new().connect(conn.as_str()).await;
            match try_pool {
                Ok(pool) => pool,
                Err(err) => {
                    println!("\x1b[31mCannot connect to MySQL pool: {err:?}\x1b[0m");
                    exit(3)
                }
            }
        }
        Err(err) => {
            println!("\x1b[31mCannot prepare database connection: {err:?}\x1b[0m");
            exit(3)
        }
    }
}