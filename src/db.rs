use cookie::Cookie;

static RASCAL_DB: &str = "rascal.sqlite3";
static CREATE_COOKIES_TABLE: &str = "
CREATE TABLE IF NOT EXISTS cookies (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    domain TEXT NOT NULL,
    path TEXT NOT NULL,
    secure BOOLEAN NOT NULL,
    http_only BOOLEAN NOT NULL,
    expiry INTEGER NOT NULL
);";

// Create a connection to the sqlite database. Will create the database
// and associated tables if they do not exist.
pub fn get_or_create_db() -> anyhow::Result<rusqlite::Connection> {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(RASCAL_DB);
    let connection = rusqlite::Connection::open(db_path)?;

    // Create the cookies table if it does not exist
    connection.execute(CREATE_COOKIES_TABLE, [])?;
    Ok(connection)
}

fn fetch_cookies<'a>(host: String, path: String) -> anyhow::Result<Vec<Cookie<'a>>> {
    let conn = get_or_create_db()?;
    let mut stmt = conn.prepare(
        "
        SELECT name, value, domain, path, secure, http_only, expiry 
        FROM cookies 
        WHERE domain = ?1 AND path ILIKE '%?2'",
    )?;
    let cookies = stmt
        .query_map([host, path], |row| {
            let name: String = row.get(0)?;
            let value: String = row.get(1)?;
            let domain: String = row.get(2)?;
            let path: String = row.get(3)?;
            let secure: bool = row.get(4)?;
            let http_only: bool = row.get(5)?;
            let expiry: i64 = row.get(6)?;
            let dt = time::OffsetDateTime::from_unix_timestamp(expiry);

            let mut cookieBuilder = Cookie::build((name, value))
                .domain(domain)
                .path(path)
                .secure(secure)
                .http_only(http_only);

            if let Ok(x) = dt {
                cookieBuilder = cookieBuilder.expires(x);
            }

            Ok(cookieBuilder.build())
        })?
        .map(|c| c.unwrap())
        .collect::<Vec<Cookie<'a>>>();
    Ok(cookies)
}
